//! RenderWare device layer on wgpu (PLAN §2 `sa-rw-wgpu`), first version for NATIVE-DEMO.
//!
//! It instances `sa-rw` objects on the GPU and draws the world pass:
//! * [`Renderer::create_texture`]: an `RwRaster` → GPU texture; DXT1/3/5 as BC1/2/3 when the
//!   adapter supports `TEXTURE_COMPRESSION_BC` (software decode otherwise, [`sa_rw::dxt`]);
//! * [`Renderer::create_geometry`]: an `RpGeometry` → vertex/index buffers plus one bind group per
//!   mesh (material colour, texture, sampler from the texture's filter/addressing);
//! * [`Renderer::render`]: sky gradient, then the main list (alpha blending on, alpha test,
//!   back-face culling per entity, `zShift` for LODs), then the sorted list back to front.
//!
//! The D3D9 state it emulates is the one `CRenderer::RenderEverythingBarRoads` and
//! `CVisibilityPlugins::RenderEntity` set (gta-reversed, not yet measured on the original);
//! what is *not* 1:1 is listed in `docs/arch/native-demo.md`.

pub mod d3d9;
pub mod sm;

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use sa_rw::{Geometry, MaterialTexture, Raster, RasterData};

/// Colour target format: a non-sRGB format, because the game blends in gamma space like D3D9.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Per-frame parameters (camera, time cycle colours).
#[derive(Debug, Clone, Copy)]
pub struct FrameParams {
    /// World → view (right-handed, camera looks down -Z).
    pub view: [[f32; 4]; 4],
    /// Vertical field of view (radians), aspect, near plane.
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub cam_pos: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
    /// `CCustomBuildingDNPipeline::m_fDNBalanceParam` (0 = day colours, 1 = night colours).
    pub dn_balance: f32,
    pub ambient: [f32; 3],
    pub dir_color: [f32; 3],
    pub light_dir: [f32; 3],
    pub sky_top: [f32; 3],
    pub sky_bottom: [f32; 3],
    /// World ambient (`amb`), added to lit prelit geometry; `ambient` is `amb_obj`.
    pub ambient_world: [f32; 3],
    /// Colour filter (`CPostEffects::ColourFilter`): `c1*a1 + c2*a2` per channel (0 = off).
    pub color_filter: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrameUniform {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    inv_view_rot_proj: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    fog_color: [f32; 4],
    fog: [f32; 4],
    ambient: [f32; 4],
    dir_color: [f32; 4],
    light_dir: [f32; 4],
    sky_top: [f32; 4],
    sky_bottom: [f32; 4],
    ambient_world: [f32; 4],
    color_filter: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    color: [f32; 4],
    flags: [u32; 4],
    surf: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [i8; 4],
    uv: [f32; 2],
    day: [u8; 4],
    night: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceData {
    model: [[f32; 4]; 4],
    extra: [f32; 4],
}

/// A texture on the GPU.
pub struct GpuTexture {
    pub view: wgpu::TextureView,
    pub has_alpha: bool,
    /// Bytes of GPU memory (approximate).
    pub bytes: u64,
}

/// One mesh (material) of a [`GpuGeometry`].
pub struct GpuMesh {
    pub first_index: u32,
    pub index_count: u32,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    flags: [u32; 4],
    surf: [f32; 4],
}

/// An `RpGeometry` on the GPU (the D3D9 driver's `RxD3D9ResEntryHeader` equivalent).
pub struct GpuGeometry {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    pub meshes: Vec<GpuMesh>,
    /// Model-space bounding sphere.
    pub bounding_sphere: [f32; 4],
    /// Bytes of GPU memory (buffers only, textures are shared).
    pub bytes: u64,
    /// Textures referenced (kept alive while the geometry lives).
    _textures: Vec<Arc<GpuTexture>>,
}

/// How an entity is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pass {
    /// The visible-entity list (`RenderEverythingBarRoads` / LOD list).
    Main,
    /// The sorted alpha list (`CVisibilityPlugins::RenderEntity`), drawn back to front.
    Sorted { additive: bool, zwrite: bool },
}

/// One entity (or atomic) to draw this frame.
pub struct DrawItem<'a> {
    pub geometry: &'a GpuGeometry,
    /// Model → world, column-major.
    pub transform: [[f32; 4]; 4],
    pub pass: Pass,
    /// Back-face culling (`CBaseModelInfo` "backface culled" flag).
    pub cull: bool,
    /// `rwRENDERSTATEALPHATESTFUNCTIONREF` (0-255).
    pub alpha_ref: f32,
    /// `RwCamera::zShift` for this entity (view units, positive = pushed back).
    pub z_shift: f32,
    /// Distance to the camera (sorting).
    pub dist: f32,
    /// Alpha multiplier (entity fade).
    pub alpha: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PipeKey {
    cull: bool,
    blend: u8, // 0 alpha, 1 additive
    zwrite: bool,
}

/// The world renderer.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    color_format: wgpu::TextureFormat,
    bc: bool,
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    material_layout: wgpu::BindGroupLayout,
    frame_buffer: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    pipelines: HashMap<PipeKey, wgpu::RenderPipeline>,
    samplers: HashMap<(u8, u8, u8), Arc<wgpu::Sampler>>,
    white: Arc<GpuTexture>,
    instances: Option<wgpu::Buffer>,
    post_layout: wgpu::BindGroupLayout,
    post_pipeline: wgpu::RenderPipeline,
    scene: Option<(wgpu::TextureView, wgpu::BindGroup, (u32, u32))>,
    pub stats: FrameStats,
}

/// Counters of the last [`Renderer::render`].
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub draw_calls: u32,
    pub instances: u32,
    pub triangles: u64,
}

/// Right-handed look-at view matrix (column-major).
pub fn look_at(eye: [f32; 3], dir: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let f = norm(dir);
    let s = norm(cross(f, up));
    let u = cross(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

fn projection(fov_y: f32, aspect: f32, near: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    [[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, 0.0, -1.0], [0.0, 0.0, near, 0.0]]
}

fn mat_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut o = [[0.0; 4]; 4];
    for (c, oc) in o.iter_mut().enumerate() {
        for (r, v) in oc.iter_mut().enumerate() {
            *v = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    o
}

/// General 4x4 inverse (column-major in, column-major out).
fn mat_inverse(m: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let a: Vec<f64> = (0..16).map(|i| m[i / 4][i % 4] as f64).collect();
    // a[c*4 + r]
    let mut inv = [0f64; 16];
    let mut w = [[0f64; 8]; 4];
    for (r, row) in w.iter_mut().enumerate() {
        for c in 0..4 {
            row[c] = a[c * 4 + r];
        }
        row[4 + r] = 1.0;
    }
    for col in 0..4 {
        let piv = (col..4).max_by(|&x, &y| w[x][col].abs().total_cmp(&w[y][col].abs())).unwrap_or(col);
        w.swap(col, piv);
        let p = w[col][col];
        if p.abs() < 1e-30 {
            return [[0.0; 4]; 4];
        }
        w[col].iter_mut().for_each(|x| *x /= p);
        let pivot_row = w[col];
        for (r, row) in w.iter_mut().enumerate() {
            if r != col {
                let f = row[col];
                row.iter_mut().zip(pivot_row).for_each(|(x, pv)| *x -= f * pv);
            }
        }
    }
    for r in 0..4 {
        for c in 0..4 {
            inv[c * 4 + r] = w[r][4 + c];
        }
    }
    let mut o = [[0.0f32; 4]; 4];
    for (i, v) in inv.iter().enumerate() {
        o[i / 4][i % 4] = *v as f32;
    }
    o
}

fn upload_texture(device: &wgpu::Device, queue: &wgpu::Queue, bc: bool, r: &Raster) -> Arc<GpuTexture> {
    let bc_format = match &r.data {
        RasterData::Bc1(_) => Some(wgpu::TextureFormat::Bc1RgbaUnorm),
        RasterData::Bc2(_) => Some(wgpu::TextureFormat::Bc2RgbaUnorm),
        RasterData::Bc3(_) => Some(wgpu::TextureFormat::Bc3RgbaUnorm),
        RasterData::Rgba8(_) => None,
    };
    let use_bc = bc_format.is_some() && bc && r.width.is_multiple_of(4) && r.height.is_multiple_of(4);
    let format = if use_bc { bc_format.unwrap_or(wgpu::TextureFormat::Rgba8Unorm) } else { wgpu::TextureFormat::Rgba8Unorm };
    let levels = r.num_levels() as u32;
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: r.width, height: r.height, depth_or_array_layers: 1 },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut bytes = 0u64;
    for i in 0..levels as usize {
        let (w, h) = sa_rw::texture::level_size(r.width, r.height, i);
        let (data, bpr, rows, extent) = if use_bc {
            let block = if matches!(r.data, RasterData::Bc1(_)) { 8 } else { 16 };
            let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
            (r.data.levels()[i].clone(), bw * block, bh, (bw * 4, bh * 4))
        } else {
            (r.level_rgba8(i), w * 4, h, (w, h))
        };
        bytes += data.len() as u64;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: i as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(rows) },
            wgpu::Extent3d { width: extent.0, height: extent.1, depth_or_array_layers: 1 },
        );
    }
    Arc::new(GpuTexture { view: tex.create_view(&Default::default()), has_alpha: r.has_alpha, bytes })
}

fn v4(v: [f32; 3], w: f32) -> [f32; 4] {
    [v[0], v[1], v[2], w]
}

impl Renderer {
    /// Device features this renderer uses when the adapter offers them.
    pub fn optional_features() -> wgpu::Features {
        wgpu::Features::TEXTURE_COMPRESSION_BC
    }

    pub fn new(device: wgpu::Device, queue: wgpu::Queue, color_format: wgpu::TextureFormat) -> Renderer {
        let bc = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sa-rw-wgpu world"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world"),
            bind_group_layouts: &[Some(&frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniform"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: frame_buffer.as_entire_binding() }],
        });
        let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky"),
            bind_group_layouts: &[Some(&frame_layout)],
            immediate_size: 0,
        });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&sky_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let post_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post"),
            bind_group_layouts: &[Some(&frame_layout), Some(&post_layout)],
            immediate_size: 0,
        });
        let post_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("colour filter"),
            layout: Some(&post_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_post"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let white_raster = Raster {
            width: 1,
            height: 1,
            format: 0,
            has_alpha: false,
            data: RasterData::Rgba8(vec![vec![255; 4]]),
        };
        let white = upload_texture(&device, &queue, bc, &white_raster);
        Renderer {
            device,
            queue,
            color_format,
            bc,
            shader,
            layout,
            material_layout,
            frame_buffer,
            frame_group,
            sky_pipeline,
            pipelines: HashMap::new(),
            samplers: HashMap::new(),
            white,
            instances: None,
            post_layout,
            post_pipeline,
            scene: None,
            stats: FrameStats::default(),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Whether DXT textures are uploaded as BC (no software decode).
    pub fn uses_bc(&self) -> bool {
        self.bc
    }

    fn pipeline(&mut self, key: PipeKey) -> &wgpu::RenderPipeline {
        if !self.pipelines.contains_key(&key) {
            let attrs0 = wgpu::vertex_attr_array![0 => Float32x3, 1 => Snorm8x4, 2 => Float32x2, 3 => Unorm8x4, 4 => Unorm8x4];
            let attrs1 = wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4];
            let blend = match key.blend {
                1 => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                },
                _ => wgpu::BlendState::ALPHA_BLENDING,
            };
            let p = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("world"),
                layout: Some(&self.layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: size_of::<Vertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &attrs0,
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: size_of::<InstanceData>() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &attrs1,
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: key.cull.then_some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(key.zwrite),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.color_format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            self.pipelines.insert(key, p);
        }
        &self.pipelines[&key]
    }

    fn sampler(&mut self, filter: u8, au: u8, av: u8) -> Arc<wgpu::Sampler> {
        self.samplers
            .entry((filter, au, av))
            .or_insert_with(|| {
                use wgpu::{FilterMode as F, MipmapFilterMode as M};
                // rwFILTER*: 1 NEAREST, 2 LINEAR, 3 MIPNEAREST, 4 MIPLINEAR, 5 LINEARMIPNEAREST,
                // 6 LINEARMIPLINEAR. Without a mip filter D3D samples level 0 only.
                let (min, mip, mips) = match filter {
                    1 => (F::Nearest, M::Nearest, false),
                    3 => (F::Nearest, M::Nearest, true),
                    4 => (F::Nearest, M::Linear, true),
                    5 => (F::Linear, M::Nearest, true),
                    6 => (F::Linear, M::Linear, true),
                    _ => (F::Linear, M::Nearest, false),
                };
                let addr = |a: u8| match a {
                    2 => wgpu::AddressMode::MirrorRepeat,
                    3 => wgpu::AddressMode::ClampToEdge,
                    4 => wgpu::AddressMode::ClampToBorder,
                    _ => wgpu::AddressMode::Repeat,
                };
                let border = self.device.features().contains(wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER);
                let fix = |m: wgpu::AddressMode| {
                    if m == wgpu::AddressMode::ClampToBorder && !border { wgpu::AddressMode::ClampToEdge } else { m }
                };
                Arc::new(self.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: None,
                    address_mode_u: fix(addr(au)),
                    address_mode_v: fix(addr(av)),
                    address_mode_w: wgpu::AddressMode::Repeat,
                    mag_filter: min,
                    min_filter: min,
                    mipmap_filter: mip,
                    lod_min_clamp: 0.0,
                    lod_max_clamp: if mips { 32.0 } else { 0.0 },
                    compare: None,
                    anisotropy_clamp: 1,
                    border_color: Some(wgpu::SamplerBorderColor::TransparentBlack),
                }))
            })
            .clone()
    }

    /// Uploads a raster.
    pub fn create_texture(&self, r: &Raster) -> Arc<GpuTexture> {
        upload_texture(&self.device, &self.queue, self.bc, r)
    }

    /// Instances a geometry. `resolve` maps a material's texture reference to an uploaded
    /// texture (`None` = missing texture: drawn untextured, as RW does with a null texture).
    pub fn create_geometry(
        &mut self,
        g: &Geometry,
        mut resolve: impl FnMut(&MaterialTexture) -> Option<Arc<GpuTexture>>,
    ) -> GpuGeometry {
        use wgpu::util::DeviceExt;
        let n = g.positions.len();
        let verts: Vec<Vertex> = (0..n)
            .map(|i| {
                let nm = g.normals.as_ref().and_then(|v| v.get(i)).copied().unwrap_or([0.0, 0.0, 1.0]);
                let q = |x: f32| (x.clamp(-1.0, 1.0) * 127.0).round() as i8;
                let day = g.prelit.as_ref().and_then(|v| v.get(i)).copied().unwrap_or([255; 4]);
                let night = g.night_colors.as_ref().and_then(|v| v.get(i)).copied().unwrap_or(day);
                Vertex {
                    pos: g.positions[i],
                    normal: [q(nm[0]), q(nm[1]), q(nm[2]), 0],
                    uv: g.uv0.as_ref().and_then(|v| v.get(i)).copied().unwrap_or([0.0; 2]),
                    day,
                    night,
                }
            })
            .collect();
        let mut indices: Vec<u32> = Vec::new();
        let mut meshes = Vec::new();
        let mut textures = Vec::new();
        for m in &g.meshes {
            if m.indices.is_empty() {
                continue;
            }
            let first = indices.len() as u32;
            indices.extend_from_slice(&m.indices);
            let mat = &g.materials[m.material];
            let tex = mat.texture.as_ref().and_then(|t| resolve(t).map(|gt| (t, gt)));
            let flags = [
                tex.is_some() as u32,
                g.modulate_material_color() as u32,
                g.prelit.is_some() as u32,
                g.normals.is_some() as u32,
            ];
            let c = mat.color;
            let sp = mat.surface_props;
            let lit = (g.flags & sa_rw::geometry::flags::LIGHT != 0) as u32 as f32;
            let surf = [sp[0], sp[1], sp[2], lit];
            let uniform = MaterialUniform {
                color: [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0],
                flags,
                surf,
            };
            let ubuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let (view_tex, sampler) = match &tex {
                Some((t, gt)) => (gt.clone(), self.sampler(t.filter(), t.address_u(), t.address_v())),
                None => (self.white.clone(), self.sampler(2, 1, 1)),
            };
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.material_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_tex.view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });
            textures.push(view_tex);
            meshes.push(GpuMesh { first_index: first, index_count: m.indices.len() as u32, bind_group, uniform: ubuf, flags, surf });
        }
        if verts.is_empty() || indices.is_empty() {
            meshes.clear();
        }
        let vb: &[u8] = if verts.is_empty() { &[0u8; 32] } else { bytemuck::cast_slice(&verts) };
        let ib: &[u8] = if indices.is_empty() { &[0u8; 4] } else { bytemuck::cast_slice(&indices) };
        let vertices = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: vb,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: ib,
            usage: wgpu::BufferUsages::INDEX,
        });
        GpuGeometry {
            bytes: (vb.len() + ib.len()) as u64,
            vertices,
            indices: indices_buf,
            meshes,
            bounding_sphere: g.bounding_sphere,
            _textures: textures,
        }
    }

    /// Changes the material colour of mesh `mesh` (e.g. water colour from the time cycle).
    pub fn set_mesh_color(&self, g: &GpuGeometry, mesh: usize, rgba: [f32; 4]) {
        if let Some(m) = g.meshes.get(mesh) {
            let u = MaterialUniform { color: rgba, flags: m.flags, surf: m.surf };
            self.queue.write_buffer(&m.uniform, 0, bytemuck::bytes_of(&u));
        }
    }

    /// Records the world pass into `encoder`: clears `target`/`depth`, draws the sky and `items`.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        size: (u32, u32),
        p: &FrameParams,
        items: &[DrawItem<'_>],
    ) {
        if self.scene.as_ref().is_none_or(|s| s.2 != size) {
            let t = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scene"),
                size: wgpu::Extent3d { width: size.0.max(1), height: size.1.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.color_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let v = t.create_view(&Default::default());
            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scene"),
                layout: &self.post_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&v) }],
            });
            self.scene = Some((v, bg, size));
        }
        let Some((scene_view, scene_group, _)) = self.scene.take() else { return };
        let proj = projection(p.fov_y, p.aspect, p.near);
        let mut view_rot = p.view;
        view_rot[3] = [0.0, 0.0, 0.0, 1.0];
        let u = FrameUniform {
            view: p.view,
            proj,
            inv_view_rot_proj: mat_inverse(&mat_mul(&proj, &view_rot)),
            cam_pos: v4(p.cam_pos, 1.0),
            fog_color: v4(p.fog_color, 1.0),
            fog: [p.fog_start, p.fog_end, p.dn_balance, p.near],
            ambient: v4(p.ambient, 0.0),
            dir_color: v4(p.dir_color, 0.0),
            light_dir: v4(p.light_dir, 0.0),
            sky_top: v4(p.sky_top, 1.0),
            sky_bottom: v4(p.sky_bottom, 1.0),
            ambient_world: v4(p.ambient_world, 0.0),
            color_filter: v4(p.color_filter, 0.0),
        };
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&u));

        // Order: main list grouped by (cull, geometry) for instancing; sorted list far to near.
        let mut main: Vec<usize> = (0..items.len()).filter(|&i| items[i].pass == Pass::Main).collect();
        main.sort_by_key(|&i| (items[i].cull, items[i].geometry as *const GpuGeometry as usize));
        let mut sorted: Vec<usize> = (0..items.len()).filter(|&i| items[i].pass != Pass::Main).collect();
        sorted.sort_by(|&a, &b| items[b].dist.total_cmp(&items[a].dist));
        let order: Vec<usize> = main.iter().chain(sorted.iter()).copied().collect();
        let inst: Vec<InstanceData> = order
            .iter()
            .map(|&i| {
                let it = &items[i];
                InstanceData { model: it.transform, extra: [it.alpha_ref, it.z_shift, it.alpha, 0.0] }
            })
            .collect();
        let need = (inst.len().max(1) * size_of::<InstanceData>()) as u64;
        if self.instances.as_ref().is_none_or(|b| b.size() < need) {
            self.instances = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size: need.next_power_of_two(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if !inst.is_empty()
            && let Some(b) = &self.instances
        {
            self.queue.write_buffer(b, 0, bytemuck::cast_slice(&inst));
        }
        // Make sure every pipeline used exists before the pass borrows self.
        let key_of = |it: &DrawItem<'_>| match it.pass {
            Pass::Main => PipeKey { cull: it.cull, blend: 0, zwrite: true },
            Pass::Sorted { additive, zwrite } => PipeKey { cull: it.cull, blend: additive as u8, zwrite },
        };
        for it in items {
            self.pipeline(key_of(it));
        }

        let mut stats = FrameStats { instances: inst.len() as u32, ..Default::default() };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &scene_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.frame_group, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
            if let Some(ib) = &self.instances {
                pass.set_vertex_buffer(1, ib.slice(..));
            }
            let mut k = 0usize;
            let mut cur_key: Option<PipeKey> = None;
            while k < order.len() {
                let it = &items[order[k]];
                let key = key_of(it);
                // run of identical geometry + pipeline (main list only)
                let mut end = k + 1;
                if it.pass == Pass::Main {
                    while end < order.len() {
                        let o = &items[order[end]];
                        if o.pass != Pass::Main || o.cull != it.cull || !std::ptr::eq(o.geometry, it.geometry) {
                            break;
                        }
                        end += 1;
                    }
                }
                if cur_key != Some(key) {
                    pass.set_pipeline(&self.pipelines[&key]);
                    cur_key = Some(key);
                }
                let g = it.geometry;
                if !g.meshes.is_empty() {
                    pass.set_vertex_buffer(0, g.vertices.slice(..));
                    pass.set_index_buffer(g.indices.slice(..), wgpu::IndexFormat::Uint32);
                    for m in &g.meshes {
                        pass.set_bind_group(1, &m.bind_group, &[]);
                        pass.draw_indexed(m.first_index..m.first_index + m.index_count, 0, k as u32..end as u32);
                        stats.draw_calls += 1;
                        stats.triangles += (m.index_count / 3) as u64 * (end - k) as u64;
                    }
                }
                k = end;
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("colour filter"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.post_pipeline);
            pass.set_bind_group(0, &self.frame_group, &[]);
            pass.set_bind_group(1, &scene_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.scene = Some((scene_view, scene_group, size));
        self.stats = stats;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_of_projection_times_view() {
        let p = projection(1.2, 1.5, 0.5);
        let v = look_at([0.0; 3], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        let m = mat_mul(&p, &v);
        let i = mat_inverse(&m);
        let id = mat_mul(&m, &i);
        for (c, col) in id.iter().enumerate() {
            for (r, v) in col.iter().enumerate() {
                assert!((v - if c == r { 1.0 } else { 0.0 }).abs() < 1e-5, "{id:?}");
            }
        }
    }

    #[test]
    fn look_at_maps_forward_to_minus_z() {
        let v = look_at([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        // point 5 units ahead
        let p = [1.0, 7.0, 3.0, 1.0];
        let z: f32 = (0..4).map(|k| v[k][2] * p[k]).sum();
        assert!((z + 5.0).abs() < 1e-5);
    }
}
