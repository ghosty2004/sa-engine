//! Translator tests: every shader runs through the WGSL translation on the GPU (vertex shaders in a
//! compute harness, pixel shaders in a fragment harness over an `Rgba32Float` target) and through
//! the reference interpreter on the CPU, on the same synthetic inputs; the results must agree.
//! Skipped without a wgpu adapter. The `exe_*` tests read the game's own shader material from the
//! user's `gta_sa.exe` at run time (`SA_GTA_EXE`, else `re/gta/1.0-US/gta_sa.exe` above the
//! crate) and are skipped when it is absent; nothing from it is stored in the repository.

use super::interp::{self, Consts, PsIn};
use super::wgsl::{translate, ConstBlock, PRELUDE};
use super::{asm, parse, Shader};
use crate::d3d9::WgpuDevice;

/// Deterministic xorshift.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }
    pub(crate) fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Uniform in [lo, hi).
    pub(crate) fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * ((self.next() >> 40) as f32 / (1u64 << 24) as f32)
    }
    pub(crate) fn v4(&mut self, lo: f32, hi: f32) -> [f32; 4] {
        [self.f(lo, hi), self.f(lo, hi), self.f(lo, hi), self.f(lo, hi)]
    }
}

pub(crate) fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let d = WgpuDevice::new_headless(1, 1);
    if d.is_none() {
        eprintln!("skipping: no wgpu adapter available");
    }
    d.map(|d| (d.device().clone(), d.queue().clone()))
}

fn close(a: f32, b: f32, tol: f32) -> bool {
    let big = |x: f32| x.is_infinite() || x.abs() >= 3.0e38;
    if big(a) || big(b) {
        return big(a) && big(b) && a.signum() == b.signum();
    }
    if a.is_nan() || b.is_nan() {
        return a.is_nan() && b.is_nan();
    }
    (a - b).abs() <= tol * 1f32.max(a.abs()).max(b.abs())
}

fn read_buffer(dev: &wgpu::Device, q: &wgpu::Queue, src: &wgpu::Buffer, size: u64) -> Vec<u8> {
    let rb = dev.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = dev.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(src, 0, &rb, 0, size);
    q.submit([enc.finish()]);
    map_read(dev, &rb)
}

fn map_read(dev: &wgpu::Device, rb: &wgpu::Buffer) -> Vec<u8> {
    let slice = rb.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = dev.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().expect("map callback").expect("map");
    let v = slice.get_mapped_range().to_vec();
    rb.unmap();
    v
}

fn storage(dev: &wgpu::Device, bytes: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytes, usage })
}

fn validate(dev: &wgpu::Device, src: &str) -> wgpu::ShaderModule {
    let scope = dev.push_error_scope(wgpu::ErrorFilter::Validation);
    let m = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(src.into()) });
    if let Some(e) = pollster_wait(scope.pop()) {
        panic!("WGSL rejected: {e}\n{}", numbered(src));
    }
    m
}

fn numbered(src: &str) -> String {
    src.lines().enumerate().map(|(i, l)| format!("{:4} {l}\n", i + 1)).collect()
}

fn pollster_wait<F: std::future::Future>(f: F) -> F::Output {
    crate::d3d9::block_on(f)
}

/// Constants used by every test: varied values, some integers and bools for flow control.
pub(crate) fn test_consts(rng: &mut Rng) -> ConstBlock {
    let mut c = ConstBlock::default();
    for v in c.vc.iter_mut() {
        *v = rng.v4(-1.5, 1.5);
    }
    for v in c.pc.iter_mut() {
        *v = rng.v4(-1.5, 1.5);
    }
    for (k, v) in c.vi.iter_mut().enumerate() {
        *v = [(k as i32 % 4) + 1, k as i32 % 3, 1 + k as i32 % 2, 0];
    }
    c.pi = c.vi;
    for k in 0..16 {
        c.vb[k] = (k % 3 == 0) as u32;
        c.pb[k] = (k % 2 == 0) as u32;
    }
    c
}

fn consts_of(c: &ConstBlock, pixel: bool) -> (Vec<[f32; 4]>, Vec<[i32; 4]>, Vec<bool>) {
    if pixel {
        (c.pc.to_vec(), c.pi.to_vec(), c.pb.iter().map(|&b| b != 0).collect())
    } else {
        (c.vc.to_vec(), c.vi.to_vec(), c.vb.iter().map(|&b| b != 0).collect())
    }
}

/// Runs vertex shader `sh` on `inputs` on the GPU; returns, per vertex, pos, d0, d1, t0..t7,
/// (fog, psize, 0, 0).
pub(crate) fn gpu_vs(dev: &wgpu::Device, q: &wgpu::Queue, sh: &Shader, consts: &ConstBlock, inputs: &[[[f32; 4]; 16]]) -> Vec<[[f32; 4]; 12]> {
    let tr = translate(sh, &|_| 2).unwrap_or_else(|e| panic!("translate: {e}\n{sh}"));
    let harness = r#"
@group(0) @binding(10) var<storage, read> cases: array<array<vec4f, 16>>;
@group(0) @binding(11) var<storage, read_write> outs: array<array<vec4f, 12>>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3u) {
    _ = sm.vb[0];
    let r = vs_body(cases[id.x]);
    var o: array<vec4f, 12>;
    o[0] = r.pos;
    o[1] = r.d0;
    o[2] = r.d1;
    for (var k = 0u; k < 8u; k++) {
        o[3u + k] = r.t[k];
    }
    o[11] = vec4f(r.fog, r.psize, 0.0, 0.0);
    outs[id.x] = o;
}
"#;
    let src = format!("{PRELUDE}\n{}\n{harness}", tr.code);
    let module = validate(dev, &src);
    let pipe = dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let n = inputs.len();
    let cbuf = storage(dev, bytemuck::bytes_of(consts), wgpu::BufferUsages::UNIFORM);
    let ibuf = storage(dev, bytemuck::cast_slice(inputs), wgpu::BufferUsages::STORAGE);
    let osize = (n * 12 * 16) as u64;
    let obuf = dev.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: osize,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 1, resource: cbuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 10, resource: ibuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 11, resource: obuf.as_entire_binding() },
        ],
    });
    let mut enc = dev.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipe);
        pass.set_bind_group(0, &bg, &[]);
        pass.dispatch_workgroups(n as u32, 1, 1);
    }
    q.submit([enc.finish()]);
    let bytes = read_buffer(dev, q, &obuf, osize);
    bytemuck::cast_slice::<u8, [[f32; 4]; 12]>(&bytes).to_vec()
}

/// Compares the GPU translation of vertex shader `src` (assembly) with the interpreter on `n`
/// random vertices (inputs in [-2, 2], `fixup` may adjust them, e.g. blend indices).
#[track_caller]
pub(crate) fn check_vs(dev: &wgpu::Device, q: &wgpu::Queue, sh: &Shader, n: usize, seed: u64, fixup: &dyn Fn(&mut [[f32; 4]; 16], &mut Rng)) {
    let mut rng = Rng::new(seed);
    let consts = test_consts(&mut rng);
    let inputs: Vec<[[f32; 4]; 16]> = (0..n)
        .map(|_| {
            let mut v = [[0.0; 4]; 16];
            for r in v.iter_mut() {
                *r = rng.v4(-2.0, 2.0);
            }
            fixup(&mut v, &mut rng);
            v
        })
        .collect();
    let got = gpu_vs(dev, q, sh, &consts, &inputs);
    let (f, i, b) = consts_of(&consts, false);
    let c = Consts { f: &f, i: &i, b: &b };
    for (k, inp) in inputs.iter().enumerate() {
        let want = interp::run_vs(sh, &c, inp).unwrap_or_else(|e| panic!("interp: {e}\n{sh}"));
        let mut w = [[0.0f32; 4]; 12];
        w[0] = want.pos;
        w[1] = want.d[0];
        w[2] = want.d[1];
        w[3..11].copy_from_slice(&want.t);
        w[11] = [want.fog, want.psize, 0.0, 0.0];
        for r in 0..12 {
            for l in 0..4 {
                assert!(
                    close(got[k][r][l], w[r][l], 2e-4),
                    "vertex {k} output {r} lane {l}: gpu {:?} cpu {:?}\ninputs {inp:?}\n{sh}",
                    got[k][r],
                    w[r]
                );
            }
        }
    }
}

/// A test texture bound to a stage: `Flat` = 4x4 RGBA8 2D, `Cube` = one RGBA8 texel per face.
#[derive(Clone)]
pub(crate) enum TestTex {
    Flat([[u8; 4]; 16]),
    Cube([[u8; 4]; 6]),
}

impl TestTex {
    pub(crate) fn random_flat(rng: &mut Rng) -> TestTex {
        let mut t = [[0u8; 4]; 16];
        for p in t.iter_mut() {
            *p = [rng.next() as u8, rng.next() as u8, rng.next() as u8, rng.next() as u8];
        }
        TestTex::Flat(t)
    }

    /// Nearest sample with clamp-to-edge (2D) / major-axis face selection (cube).
    pub(crate) fn sample(&self, c: [f32; 4]) -> [f32; 4] {
        let n = |p: [u8; 4]| p.map(|x| x as f32 / 255.0);
        match self {
            TestTex::Flat(t) => {
                let x = ((c[0] * 4.0).floor() as i32).clamp(0, 3) as usize;
                let y = ((c[1] * 4.0).floor() as i32).clamp(0, 3) as usize;
                n(t[y * 4 + x])
            }
            TestTex::Cube(f) => {
                let (ax, ay, az) = (c[0].abs(), c[1].abs(), c[2].abs());
                // ties go to z, then y (Vulkan's major-axis rule)
                let face = if az >= ax && az >= ay {
                    if c[2] >= 0.0 { 4 } else { 5 }
                } else if ay >= ax {
                    if c[1] >= 0.0 { 2 } else { 3 }
                } else if c[0] >= 0.0 {
                    0
                } else {
                    1
                };
                n(f[face])
            }
        }
    }
}

const PS_W: u32 = 16;

/// Runs pixel shader `sh` per case on the GPU: `Some(colour)` or `None` when discarded.
pub(crate) fn gpu_ps(dev: &wgpu::Device, q: &wgpu::Queue, sh: &Shader, consts: &ConstBlock, texs: &[(u32, TestTex)], cases: &[PsIn]) -> Vec<Option<[f32; 4]>> {
    let kind = |s: u32| match texs.iter().find(|t| t.0 == s) {
        Some((_, TestTex::Cube(_))) => 3,
        _ => 2,
    };
    let tr = translate(sh, &kind).unwrap_or_else(|e| panic!("translate: {e}\n{sh}"));
    let n = cases.len() as u32;
    let h = n.div_ceil(PS_W);
    let harness = format!(
        r#"
@group(0) @binding(10) var<storage, read> cases: array<array<vec4f, 10>>;
@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {{
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}}
@fragment
fn fs(@builtin(position) p: vec4f) -> @location(0) vec4f {{
    let idx = min(u32(p.y) * {PS_W}u + u32(p.x), {last}u);
    _ = sm.pb[0];
    let c = cases[idx];
    let r = ps_body(c[0], c[1], array<vec4f, 8>(c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9]));
    return r.color;
}}
"#,
        last = n - 1
    );
    let src = format!("{PRELUDE}\n{}\n{harness}", tr.code);
    let module = validate(dev, &src);
    let pipe = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_full"), compilation_options: Default::default(), buffers: &[] },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba32Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let flat: Vec<[[f32; 4]; 10]> = cases
        .iter()
        .map(|c| {
            let mut a = [[0.0; 4]; 10];
            a[0] = c.d[0];
            a[1] = c.d[1];
            a[2..].copy_from_slice(&c.t);
            a
        })
        .collect();
    let cbuf = storage(dev, bytemuck::bytes_of(consts), wgpu::BufferUsages::UNIFORM);
    let ibuf = storage(dev, bytemuck::cast_slice(&flat), wgpu::BufferUsages::STORAGE);
    let bg0 = dev.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipe.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 1, resource: cbuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 10, resource: ibuf.as_entire_binding() },
        ],
    });
    // textures of the sampled stages
    let sampler = dev.create_sampler(&wgpu::SamplerDescriptor::default());
    let mut views = Vec::new();
    for &(s, _) in &tr.samplers {
        let t = texs.iter().find(|t| t.0 == s).map(|t| t.1.clone()).unwrap_or(TestTex::Flat([[255; 4]; 16]));
        let (size, layers, data, dim): (u32, u32, Vec<u8>, wgpu::TextureViewDimension) = match &t {
            TestTex::Flat(p) => (4, 1, p.iter().flatten().copied().collect(), wgpu::TextureViewDimension::D2),
            TestTex::Cube(f) => (1, 6, f.iter().flatten().copied().collect(), wgpu::TextureViewDimension::Cube),
        };
        let tex = dev.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        q.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
        );
        views.push((s, tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(dim), ..Default::default() })));
    }
    let mut entries = Vec::new();
    for (s, v) in &views {
        entries.push(wgpu::BindGroupEntry { binding: 2 * s, resource: wgpu::BindingResource::TextureView(v) });
        entries.push(wgpu::BindGroupEntry { binding: 2 * s + 1, resource: wgpu::BindingResource::Sampler(&sampler) });
    }
    let bg1 = (!views.is_empty()).then(|| dev.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &pipe.get_bind_group_layout(1), entries: &entries }));
    let target = dev.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: PS_W, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    const KILLED: f64 = -12345.0;
    let mut enc = dev.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: KILLED, g: KILLED, b: KILLED, a: KILLED }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipe);
        pass.set_bind_group(0, &bg0, &[]);
        if let Some(bg1) = &bg1 {
            pass.set_bind_group(1, bg1, &[]);
        }
        pass.draw(0..3, 0..1);
    }
    let rb = dev.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (PS_W * 16 * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &rb, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(PS_W * 16), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: PS_W, height: h, depth_or_array_layers: 1 },
    );
    q.submit([enc.finish()]);
    let bytes = map_read(dev, &rb);
    let px: &[[f32; 4]] = bytemuck::cast_slice(&bytes);
    (0..n as usize).map(|i| if px[i][0] == KILLED as f32 { None } else { Some(px[i]) }).collect()
}

/// Compares the GPU translation of pixel shader `sh` with the interpreter on `n` random pixels.
#[track_caller]
pub(crate) fn check_ps(dev: &wgpu::Device, q: &wgpu::Queue, sh: &Shader, texs: &[(u32, TestTex)], n: usize, seed: u64, tc_range: (f32, f32)) {
    let mut rng = Rng::new(seed);
    let consts = test_consts(&mut rng);
    let cases: Vec<PsIn> = (0..n)
        .map(|_| {
            let mut c = PsIn { d: [rng.v4(0.0, 1.0), rng.v4(0.0, 1.0)], ..Default::default() };
            for t in c.t.iter_mut() {
                // texel centres of the 4x4 test textures keep nearest sampling unambiguous
                *t = [0, 1, 2, 3].map(|_| {
                    let x = rng.f(tc_range.0, tc_range.1);
                    ((x * 4.0).floor() + 0.5) / 4.0
                });
            }
            c
        })
        .collect();
    let got = gpu_ps(dev, q, sh, &consts, texs, &cases);
    let (f, i, b) = consts_of(&consts, true);
    let c = Consts { f: &f, i: &i, b: &b };
    let sample = |s: u32, coord: [f32; 4]| match texs.iter().find(|t| t.0 == s) {
        Some((_, t)) => t.sample(coord),
        None => [1.0; 4],
    };
    for (k, case) in cases.iter().enumerate() {
        let want = interp::run_ps(sh, &c, case, &sample).unwrap_or_else(|e| panic!("interp: {e}\n{sh}"));
        match (got[k], want) {
            (None, None) => {}
            (Some(g), Some(w)) => {
                for l in 0..4 {
                    assert!(close(g[l], w.color[l], 2e-4), "pixel {k} lane {l}: gpu {g:?} cpu {:?}\ninput {case:?}\n{sh}", w.color);
                }
            }
            (g, w) => panic!("pixel {k}: gpu {g:?} cpu {w:?} (None = killed)\ninput {case:?}\n{sh}"),
        }
    }
}

pub(crate) fn assemble(src: &str) -> Shader {
    let t = asm::assemble(src).unwrap_or_else(|e| panic!("assemble: {e}\n{src}"));
    parse(&t).unwrap_or_else(|e| panic!("parse: {e}\n{src}"))
}

// ------------------------------------------------------------------------------- tests -------

/// RW-style skinning (two bones through `a0`), lighting and fog in vs_1_1.
const VS_SKIN_11: &str = "vs_1_1
dcl_position0 v0
dcl_blendweight0 v1
dcl_blendindices0 v2
dcl_normal0 v3
dcl_color0 v4
dcl_texcoord0 v5
mov a0.x, v2.x
mul r8, v1.x, c[a0.x + 30]
mul r9, v1.x, c[a0.x + 31]
mul r10, v1.x, c[a0.x + 32]
mov a0.x, v2.y
mad r8, v1.y, c[a0.x + 30], r8
mad r9, v1.y, c[a0.x + 31], r9
mad r10, v1.y, c[a0.x + 32], r10
m4x3 r3.xyz, v0, r8
mov r3.w, v0.w
m3x3 r5.xyz, v3, r8
dp3 r5.w, r5, r5
rsq r5.w, r5.w
mul r5.xyz, r5, r5.w
m4x4 oPos.xyzw, r3, c[0]
dp4 r0.w, r3, c[3]
mul r0.w, r0.w, c[4].x
mul r0.w, r0.w, r0.w
expp r0.w, r0.w
rcp oFog, r0.w
add r11, v4, c[4]
dp3 r6.x, c[10], r5
max r6.x, c[5].w, -r6.x
mad oD0, c[12], r6.x, r11
slt r4.w, v1.w, v1.w
mov oT0.xy, v5
";

#[test]
fn vs_skin_lighting_fog() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(VS_SKIN_11);
    check_vs(&dev, &q, &sh, 64, 1, &|v, rng| {
        // blend indices: integral bone offsets (3 rows per bone), 0..8 bones
        v[2] = [0, 1, 2, 3].map(|_| (3 * (rng.next() % 8)) as f32 + 0.25);
        v[0][3] = 1.0;
    });
}

/// vs_2_0 with `mova` (round to nearest), `a0.y` relative reads, swizzled cross products.
const VS_20: &str = "vs_2_0
dcl_position0 v0
dcl_blendindices0 v2
dcl_normal0 v3
dcl_tangent0 v13
mova a0.xyzw, v2
m4x3 r5.xyz, v0, c[a0.y + 40]
m3x3 r4.xyz, v13, c[a0.z + 40]
mul r2, r4.zxyw, v3.yzxw
mad r2, r4.yzxw, v3.zxyw, -r2
dp3 r2.w, r2, r2
rsq r2.w, r2.w
mul r2.xyz, r2, r2.w
add r7.xyz, -c[1 + 5].xyz, r5.xyz
dp3 r1.x, r7, r7
rsq r1.x, r1.x
mul r7.xyz, r7.xyz, r1.x
dp3 oT2.x, c[20], r4
dp3 oT2.y, c[21], r2
dp3 oT2.z, c[22], r7
mad oT3.xyz, r2, c[5].w, c[5].w
mov r5.w, v0.w
m4x4 oPos, r5, c[0]
mov oD1, c[a0.w + 40]
";

#[test]
fn vs_20_relative_and_mova() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(VS_20);
    check_vs(&dev, &q, &sh, 64, 2, &|v, rng| {
        // mova rounding: halves must not sit at x.5 exactly where f32 noise flips the result
        v[2] = [0, 1, 2, 3].map(|_| (rng.next() % 20) as f32 + rng.f(-0.4, 0.4));
    });
}

/// Every ALU op of vs_2_0 plus `def`/`defi`/`defb`, static flow control and `aL` addressing.
const VS_ALU: &str = "vs_2_0
def c100, 0.5, -2.0, 3.25, 0.0
defi i3, 3, 7, 2, 0
defb b5, true
dcl_position0 v0
dcl_texcoord0 v5
dcl_texcoord1 v6
mov r0, v0
lit r1, v5
dst r2, v5, v6
lrp r3, v5.x, v0, v6
frc r4, v6
pow r5.x, v5.x, v6.y
sgn r6, v0, r10, r11
abs r7, v6
nrm r8, v5
sincos r9.xy, v6.z, c[90], c[91]
exp r10.y, v5.x
log r10.z, v6.w
logp r10.w, v6.x
expp r10.x, v0.y
min r11, v0, v5
max r11.zw, v0, -v6
sge r1.zw, v5, v6
crs r2.xyz, v5, v6
mad r3, v5, c100, c100.z
rep i3
add r0, r0, c100.x
endrep
loop aL, i3
mad r4, r4, c[aL + 7], c[aL + 3]
endloop
if b5
mul r5, r5, c100.y
else
mul r5, r5, c100.x
endif
if b3
add r6, r6, c[2]
endif
if_gt v5.x, v6.x
mov r7, -r7
endif
rep i1
add r8, r8, c[1]
break_ge r8.x, c100.w
endrep
m4x4 oPos, r0, c[0]
mov oD0, r1
mov oD1, r2
mov oT0, r3
mov oT1, r4
mov oT2, r5
mov oT3, r6
mov oT4, r7
mov oT5, r8
mov oT6, r9
mov oT7, r10
mov oFog, r11.y
mov oPts, r11.z
";

#[test]
fn vs_alu_and_flow_control() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(VS_ALU);
    check_vs(&dev, &q, &sh, 64, 3, &|_, _| {});
}

#[test]
fn vs_11_logp_expp_rcp_special_values() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(
        "vs_1_1
dcl_position0 v0
dcl_texcoord0 v5
expp oT0, v5.x
logp oT1, v5.y
rcp oT2.x, v5.z
rsq oT2.y, v5.z
rcp oT2.z, v5.w
log oT2.w, v5.w
mov oPos, v0
",
    );
    check_vs(&dev, &q, &sh, 32, 4, &|v, rng| {
        // exercise the exact special cases: 0 and 1
        let pick = |rng: &mut Rng| match rng.next() % 3 {
            0 => 0.0,
            1 => 1.0,
            _ => rng.f(-3.0, 3.0),
        };
        v[5] = [pick(rng), pick(rng), pick(rng), pick(rng)];
    });
}

/// ps_1_1: modifiers, coissue, cnd/lrp/dp3 with _bx2, texcoord, texkill, shifts, saturation,
/// constant clamping, three texture stages.
const PS_11: &str = "ps_1_1
def c7, 0.25, -0.5, 1.75, 0.5
tex t0
tex t1
texcoord t2
texkill t3
mul r0, t0, 1-t0.a
mul_x2 r0, r0, v0
+mov_sat r0.a, c7.z
dp3_sat r1, t1_bx2, v1_bx2
lrp r1.rgb, t2, r1, c3
cnd r0.rgb, r0.a, r0, r1
+mul r0.a, t1.a, v0_bias
mad_d2 r0.rgb, r0, c2, -r1
";

#[test]
fn ps_11_modifiers_coissue_texkill() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(PS_11);
    let mut rng = Rng::new(10);
    let texs = [(0, TestTex::random_flat(&mut rng)), (1, TestTex::random_flat(&mut rng))];
    // texcoords in [-0.25, 1): t3 kills about a fifth of the pixels
    check_ps(&dev, &q, &sh, &texs, 160, 11, (-0.25, 1.0));
}

/// ps_1_4: texld from texcoords and (phase 2) from computed registers, texcrd, phase.
const PS_14: &str = "ps_1_4
texcrd r1.rgb, t1.xyz
texld r0, t0
mul r1.rgb, r1, c1
phase
texld r2, r1
mul r0, r0, r2
add_sat r0.rgb, r0, v0
";

#[test]
fn ps_14_phases() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(PS_14);
    let mut rng = Rng::new(20);
    let texs = [(0, TestTex::random_flat(&mut rng)), (2, TestTex::random_flat(&mut rng))];
    check_ps(&dev, &q, &sh, &texs, 64, 21, (0.0, 1.0));
}

/// ps_2_0: dcl, texld/texldp, a cube sampler, cmp, dp2add, oC0.
const PS_20: &str = "ps_2_0
def c9, 0.5, 2.0, -1.0, 0.0
dcl t0.xy
dcl t1
dcl v0
dcl_2d s0
dcl_cube s3
texld r0, t0, s0
texldp r1, t1, s0
mad r2.xyz, t1, c9.y, c9.z
texld r2, r2, s3
cmp r3, r2_bias, r0, r1
dp2add r4.x, r3, c1, c9.x
mul r3, r3, r4.x
lrp r3.rgb, v0, r3, r2
mov_sat oC0, r3
";

#[test]
fn ps_20_samplers_cube() {
    let Some((dev, q)) = gpu() else { return };
    let sh = assemble(PS_20);
    let mut rng = Rng::new(30);
    let cube = TestTex::Cube([[255, 0, 0, 255], [0, 255, 0, 128], [0, 0, 255, 64], [255, 255, 0, 255], [0, 255, 255, 0], [255, 0, 255, 200]]);
    let texs = [(0, TestTex::random_flat(&mut rng)), (3, cube)];
    // t1.w around 1 so texldp lands near texel centres; the comparison tolerates the rest
    check_ps(&dev, &q, &sh, &texs, 96, 31, (0.0, 1.0));
}

// --------------------------------------------------------------- the game's own material ---

/// The user's `gta_sa.exe` (1.0 US), if present.
pub(crate) fn exe() -> Option<Vec<u8>> {
    let path = std::env::var_os("SA_GTA_EXE").map(std::path::PathBuf::from).or_else(|| {
        let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            let p = d.join("re/gta/1.0-US/gta_sa.exe");
            if p.exists() {
                return Some(p);
            }
            if !d.pop() {
                return None;
            }
        }
    });
    let data = path.and_then(|p| std::fs::read(p).ok());
    if data.is_none() {
        eprintln!("skipping: gta_sa.exe not found (set SA_GTA_EXE)");
    }
    data
}

/// Reads `len` bytes at virtual address `va` of a PE image file.
pub(crate) fn pe_read(img: &[u8], va: u32, len: usize) -> Option<&[u8]> {
    let u16_at = |o: usize| img.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| img.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let pe = u32_at(0x3C)? as usize;
    let nsec = u16_at(pe + 6)? as usize;
    let opt = u16_at(pe + 20)? as usize;
    let base = u32_at(pe + 24 + 28)?;
    let rva = va.checked_sub(base)?;
    for k in 0..nsec {
        let s = pe + 24 + opt + 40 * k;
        let (vsize, vaddr, rsize, raddr) = (u32_at(s + 8)?, u32_at(s + 12)?, u32_at(s + 16)?, u32_at(s + 20)?);
        if rva >= vaddr && rva < vaddr + vsize.max(rsize) {
            let off = (raddr + (rva - vaddr)) as usize;
            return img.get(off..off + len);
        }
    }
    None
}

/// Token stream at `va` (through its end token).
pub(crate) fn exe_tokens(img: &[u8], va: u32) -> Option<Vec<u32>> {
    let word = |i: usize| pe_read(img, va + 4 * i as u32, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let n = parse::token_length(word)?;
    (0..n).map(word).collect()
}

/// RpMatFX's six `ps_1_1` shaders (`_rpMatFXPipelinesCreate` 0x815E60 passes these addresses to
/// `RwD3D9CreatePixelShader`).
pub(crate) const MATFX_PS: [u32; 6] = [0x8852F0, 0x885350, 0x8853A0, 0x8853E0, 0x885468, 0x885518];

#[test]
fn exe_matfx_pixel_shaders() {
    let Some(img) = exe() else { return };
    let gpu = gpu();
    let mut rng = Rng::new(40);
    let texs = [(0, TestTex::random_flat(&mut rng)), (1, TestTex::random_flat(&mut rng)), (2, TestTex::random_flat(&mut rng))];
    for (k, &va) in MATFX_PS.iter().enumerate() {
        let tokens = exe_tokens(&img, va).unwrap_or_else(|| panic!("no shader at {va:#x}"));
        let sh = parse(&tokens).unwrap_or_else(|e| panic!("{va:#x}: {e}"));
        assert_eq!(sh.version.to_string(), "ps_1_1");
        // the assembler reproduces D3DX's encoding of the disassembly exactly
        assert_eq!(asm::assemble(&sh.to_string()).map_err(|e| e.to_string()), Ok(tokens.clone()), "{va:#x} round trip");
        if let Some((dev, q)) = &gpu {
            check_ps(dev, q, &sh, &texs, 64, 41 + k as u64, (0.0, 1.0));
        }
    }
}

/// Instantiates one of `_rpD3D9VertexShaderGenerate`'s format strings with plausible arguments.
fn fill_format(f: &str, args: &mut dyn Iterator<Item = u32>) -> String {
    let mut out = String::new();
    let mut it = f.chars().peekable();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('d') | Some('u') => {
                let a = args.next().unwrap_or(0);
                // register numbers (`v%u`, `oT%u`) stay small; constant offsets may be larger
                let a = if out.ends_with('v') || out.ends_with('T') { a % 4 } else { a };
                out += &a.to_string();
            }
            // `%c` selects a component (`v1.%c`, `a0.%c`)
            Some('c') => out.push(['x', 'y', 'z', 'w'][(args.next().unwrap_or(0) % 4) as usize]),
            // `%s`: a register name, or after a '.' a write mask / swizzle
            Some('s') => out += if out.ends_with('.') { "xy" } else { "r5" },
            Some(o) => {
                out.push('%');
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out
}

/// The vertex-shader generator's format strings (`.data` 0x8D641C..0x8D7354, read at run time).
fn exe_vs_formats(img: &[u8]) -> Vec<String> {
    let (start, end) = (0x8D641Cu32, 0x8D7354u32);
    let bytes = pe_read(img, start, (end - start) as usize).expect("format table");
    let mut v = Vec::new();
    for s in bytes.split(|&b| b == 0) {
        if let Ok(s) = std::str::from_utf8(s)
            && s.ends_with('\n')
            && !s.starts_with("dcl")
            && !s.starts_with("vs_")
        {
            v.push(s.trim_end().to_string());
        }
    }
    v
}

#[test]
fn exe_rw_vertex_shader_fragments() {
    let Some(img) = exe() else { return };
    let formats = exe_vs_formats(&img);
    assert!(formats.len() > 100, "{} format strings", formats.len());
    let gpu = gpu();
    for (version, skip) in [("vs_1_1", "mova"), ("vs_2_0", "mov a0")] {
        let mut src = format!("{version}\n");
        for d in ["position0 v0", "blendweight0 v1", "blendindices0 v2", "normal0 v3", "color0 v4", "texcoord0 v5", "texcoord1 v6", "tangent0 v13"] {
            src += &format!("dcl_{d}\n");
        }
        let mut n = 0;
        let mut args = (1..).map(|k: u32| 8 + (k * 7) % 40);
        for f in &formats {
            if f.starts_with(skip) || (version == "vs_1_1" && (f.contains("a0.%c") || f.contains("a0.%s"))) {
                continue;
            }
            // relative reads stay in range: a0 is loaded from blend indices 0..2
            let line = fill_format(f, &mut args);
            src += &line;
            src.push('\n');
            n += 1;
        }
        let sh = assemble(&src);
        assert!(n > 80, "{version}: {n} lines");
        if let Some((dev, q)) = &gpu {
            check_vs(dev, q, &sh, 32, 50, &|v, rng| {
                v[2] = [0, 1, 2, 3].map(|_| (rng.next() % 3) as f32);
            });
        }
    }
}
