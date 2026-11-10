//! The viewer core: camera, visibility with the LOD hierarchy, streaming state, draw list.
//! Shared by the window, screenshot and benchmark front ends. Viewer code, not ported game
//! code (the game's `CRenderer`/`CStreaming` are not ported yet); the rules it approximates are
//! named where they are used and listed in `docs/arch/native-demo.md`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use sa_platform::fs::GameFs;
use sa_rw::{Clump, Geometry, Material, MaterialTexture, Mesh, RwMatrix, TexDictionary};
use sa_rw_wgpu::{DrawItem, FrameParams, GpuGeometry, GpuTexture, Pass, Renderer};

use crate::loader::{Loaded, Loader, Request};
use crate::timecycle::{TimeCycle, dn_balance};
use crate::world::{ModelKind, World};

/// Free-fly camera. `yaw` 0 looks north (+Y), positive turns right (clockwise); `pitch` up.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    /// Vertical field of view (radians).
    pub fov_y: f32,
}

impl Camera {
    pub fn dir(&self) -> [f32; 3] {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        [self.yaw.sin() * cp, self.yaw.cos() * cp, sp]
    }

    pub fn right(&self) -> [f32; 3] {
        [self.yaw.cos(), -self.yaw.sin(), 0.0]
    }

    pub fn look_dir(&mut self, d: [f32; 3]) {
        let h = (d[0] * d[0] + d[1] * d[1]).sqrt();
        self.yaw = d[0].atan2(d[1]);
        self.pitch = d[2].atan2(h);
    }
}

/// A model on the GPU: its atomics with their frame matrices.
pub struct GpuModel {
    /// (atomic → model matrix, geometry)
    pub parts: Vec<([[f32; 4]; 4], GpuGeometry)>,
    /// Bounding sphere in model space.
    pub sphere: [f32; 4],
    pub bytes: u64,
}

enum Slot<T> {
    None,
    Requested,
    Ready(T),
    Failed,
}

struct ModelSlot {
    state: Slot<Arc<GpuModel>>,
    last_used: u64,
}

struct TxdSlot {
    state: Slot<Arc<HashMap<String, Arc<GpuTexture>>>>,
    last_used: u64,
    bytes: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ViewerStats {
    pub visible: usize,
    pub models_loaded: usize,
    pub txds_loaded: usize,
    pub pending: usize,
    pub gpu_mb: f64,
    pub draw_calls: u32,
    pub triangles: u64,
    pub evicted: usize,
}

pub struct Options {
    pub draw_dist_scale: f32,
    pub budget_bytes: u64,
    pub area: u8,
    pub max_in_flight: usize,
    /// Upload time per frame in interactive mode (seconds).
    pub upload_budget: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options { draw_dist_scale: 1.0, budget_bytes: 1536 << 20, area: 0, max_in_flight: 96, upload_budget: 0.006 }
    }
}

pub struct Viewer {
    pub world: World,
    tc: TimeCycle,
    pub renderer: Renderer,
    loader: Loader,
    models: Vec<ModelSlot>,
    txds: Vec<TxdSlot>,
    parked: Vec<(usize, Clump)>,
    roots: Vec<usize>,
    water: Option<GpuGeometry>,
    pub cam: Camera,
    /// Minutes since midnight.
    pub minutes: f32,
    pub weather: usize,
    pub opts: Options,
    frame: u64,
    gpu_bytes: u64,
    pub stats: ViewerStats,
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
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

fn max_scale(m: &[[f32; 4]; 4]) -> f32 {
    (0..3).map(|c| (m[c][0].powi(2) + m[c][1].powi(2) + m[c][2].powi(2)).sqrt()).fold(0.0, f32::max)
}

/// `CClock::GetIsTimeInRange(on, off)`.
fn time_in_range(hour: u8, on: u8, off: u8) -> bool {
    if on > off { hour >= on || hour < off } else { hour >= on && hour < off }
}

/// Atomic selection of a model's DFF (viewer rule, see native-demo.md): damaged (`_dam`) and
/// very-low (`_vlo`) atomics are skipped; atomic models ignore the DFF frame matrices (the game
/// gives each instance a new frame, `CAtomicModelInfo::CreateInstance`).
fn select_atomics(c: &Clump, kind: ModelKind) -> Vec<(RwMatrix, Arc<Geometry>)> {
    let skip = |name: &str| name.ends_with("_dam") || name.ends_with("_vlo");
    let mut out: Vec<(RwMatrix, Arc<Geometry>)> = c
        .atomics
        .iter()
        .filter(|a| !skip(&c.frames[a.frame].name))
        .map(|a| {
            let m = if kind == ModelKind::Clump { c.frame_ltm(a.frame) } else { RwMatrix::IDENTITY };
            (m, a.geometry.clone())
        })
        .collect();
    if kind != ModelKind::Clump && out.len() > 1 {
        // CAtomicModelInfo holds one atomic: the first undamaged one.
        out.truncate(1);
    }
    if out.is_empty()
        && let Some(a) = c.atomics.first()
    {
        out.push((RwMatrix::IDENTITY, a.geometry.clone()));
    }
    out
}

impl Viewer {
    pub fn new(fs: Arc<dyn GameFs>, world: World, renderer: Renderer, opts: Options) -> Viewer {
        let tc = TimeCycle::parse(&world.timecyc);
        let loader = Loader::new(fs.clone(), world.archive_paths(), 3);
        let models = (0..world.models.len()).map(|_| ModelSlot { state: Slot::None, last_used: 0 }).collect();
        let txds = (0..world.txds.len()).map(|_| TxdSlot { state: Slot::None, last_used: 0, bytes: 0 }).collect();
        let roots = (0..world.instances.len())
            .filter(|&i| world.instances[i].lod_parent.is_none() && world.instances[i].model != usize::MAX)
            .collect();
        let mut v = Viewer {
            world,
            tc,
            renderer,
            loader,
            models,
            txds,
            parked: Vec::new(),
            roots,
            water: None,
            cam: Camera { pos: [2495.0, -1670.0, 20.0], yaw: 0.0, pitch: 0.0, fov_y: 55.4f32.to_radians() },
            minutes: 12.0 * 60.0,
            weather: 0,
            opts,
            frame: 0,
            gpu_bytes: 0,
            stats: ViewerStats::default(),
        };
        v.build_water(fs.as_ref());
        v
    }

    fn build_water(&mut self, fs: &dyn GameFs) {
        if self.world.water.is_empty() {
            return;
        }
        let (mut pos, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new());
        for p in &self.world.water {
            let b = pos.len() as u32;
            for v in &p.verts {
                pos.push(*v);
                uv.push([v[0] / 32.0, v[1] / 32.0]);
            }
            // water.dat quads: v0 (x0,y0), v1 (x1,y0), v2 (x0,y1), v3 (x1,y1)
            idx.extend_from_slice(&[b, b + 1, b + 2]);
            if p.verts.len() == 4 {
                idx.extend_from_slice(&[b + 1, b + 3, b + 2]);
            }
        }
        let n = pos.len();
        let tex = MaterialTexture { name: "waterclear256".into(), mask: String::new(), filter_addressing: 0x1106 };
        let g = Geometry {
            flags: sa_rw::geometry::flags::MODULATE_MATERIAL_COLOR | sa_rw::geometry::flags::PRELIT,
            num_vertices: n,
            bounding_sphere: [0.0, 0.0, 0.0, 1e9],
            positions: pos,
            normals: None,
            uv0: Some(uv),
            prelit: Some(vec![[255; 4]; n]),
            night_colors: None,
            materials: vec![Material { color: [255; 4], surface_props: [1.0; 3], texture: Some(tex), has_effects: false }],
            meshes: vec![Mesh { material: 0, indices: idx }],
            tristrip: false,
        };
        // CWaterLevel uses "waterclear256" from particle.txd
        let water_tex = fs
            .read("MODELS\\PARTICLE.TXD")
            .ok()
            .and_then(|d| sa_rw::read_tex_dictionary(&d).ok())
            .and_then(|d| d.find("waterclear256").map(|t| self.renderer.create_texture(&t.raster)));
        self.water = Some(self.renderer.create_geometry(&g, |_| water_tex.clone()));
    }

    pub fn num_weathers(&self) -> usize {
        self.tc.num_weathers()
    }

    pub fn hour(&self) -> u8 {
        ((self.minutes / 60.0) as u32 % 24) as u8
    }

    fn model_ready(&self, inst: usize) -> bool {
        let m = self.world.instances[inst].model;
        matches!(self.models[m].state, Slot::Ready(_) | Slot::Failed)
    }

    fn txd_chain(&self, txd: Option<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut t = txd;
        while let Some(i) = t {
            if out.contains(&i) {
                break;
            }
            out.push(i);
            t = self.world.txds[i].parent;
        }
        out
    }

    /// Handles finished loads and uploads parked models whose dictionaries are ready.
    /// Returns the number of uploads done.
    fn pump(&mut self, sync: bool) -> usize {
        let start = Instant::now();
        let mut done = 0;
        loop {
            if !sync && start.elapsed().as_secs_f64() > self.opts.upload_budget {
                break;
            }
            let Some(l) = self.loader.poll(false) else { break };
            match l {
                Loaded::Txd { txd, result } => match result {
                    Ok(d) => self.upload_txd(txd, &d),
                    Err(e) => {
                        eprintln!("txd {}: {e}", self.world.txds[txd].name);
                        self.txds[txd].state = Slot::Failed;
                    }
                },
                Loaded::Model { model, result } => match result {
                    Ok(c) => self.parked.push((model, c)),
                    Err(e) => {
                        eprintln!("model {}: {e}", self.world.models[model].name);
                        self.models[model].state = Slot::Failed;
                    }
                },
            }
            done += 1;
        }
        // Parked clumps: request missing dictionaries, upload when the chain is resolved.
        let parked = std::mem::take(&mut self.parked);
        for (model, clump) in parked {
            let chain = self.txd_chain(self.world.models[model].txd);
            let mut ready = true;
            for &t in &chain {
                match self.txds[t].state {
                    Slot::None => {
                        ready = false;
                        match self.world.txds[t].entry {
                            Some(entry) => {
                                self.txds[t].state = Slot::Requested;
                                self.loader.request(Request::Txd { txd: t, entry });
                            }
                            None => self.txds[t].state = Slot::Failed,
                        }
                    }
                    Slot::Requested => ready = false,
                    _ => {}
                }
            }
            if !ready || (!sync && start.elapsed().as_secs_f64() > self.opts.upload_budget) {
                self.parked.push((model, clump));
                continue;
            }
            self.upload_model(model, &clump, &chain);
            done += 1;
        }
        done
    }

    fn upload_txd(&mut self, txd: usize, d: &TexDictionary) {
        let mut map = HashMap::new();
        let mut bytes = 0;
        for t in &d.textures {
            let g = self.renderer.create_texture(&t.raster);
            bytes += g.bytes;
            map.insert(t.name.clone(), g);
        }
        self.gpu_bytes += bytes;
        self.txds[txd] = TxdSlot { state: Slot::Ready(Arc::new(map)), last_used: self.frame, bytes };
    }

    fn upload_model(&mut self, model: usize, c: &Clump, chain: &[usize]) {
        let kind = self.world.models[model].kind;
        let dicts: Vec<Arc<HashMap<String, Arc<GpuTexture>>>> = chain
            .iter()
            .filter_map(|&t| match &self.txds[t].state {
                Slot::Ready(m) => Some(m.clone()),
                _ => None,
            })
            .collect();
        let mut parts = Vec::new();
        let mut bytes = 0;
        let mut sphere = [0.0f32; 4];
        for (m, g) in select_atomics(c, kind) {
            let gg = self.renderer.create_geometry(&g, |t| dicts.iter().find_map(|d| d.get(&t.name).cloned()));
            bytes += gg.bytes;
            let s = g.bounding_sphere;
            let ctr = m.transform_point([s[0], s[1], s[2]]);
            let r = (ctr[0].powi(2) + ctr[1].powi(2) + ctr[2].powi(2)).sqrt() + s[3] * m.max_scale();
            sphere[3] = sphere[3].max(r);
            parts.push((m.to_cols(), gg));
        }
        self.gpu_bytes += bytes;
        self.models[model] = ModelSlot { state: Slot::Ready(Arc::new(GpuModel { parts, sphere, bytes })), last_used: self.frame };
    }

    fn request_model(&mut self, model: usize) -> bool {
        if !matches!(self.models[model].state, Slot::None) {
            return false;
        }
        match self.world.models[model].dff {
            Some(entry) => {
                self.models[model].state = Slot::Requested;
                self.loader.request(Request::Model { model, entry });
                true
            }
            None => {
                self.models[model].state = Slot::Failed;
                false
            }
        }
    }

    fn evict(&mut self) {
        if self.gpu_bytes <= self.opts.budget_bytes {
            return;
        }
        let target = self.opts.budget_bytes / 10 * 9;
        let mut cands: Vec<(u64, bool, usize)> = Vec::new();
        for (i, m) in self.models.iter().enumerate() {
            if matches!(m.state, Slot::Ready(_)) && m.last_used + 30 < self.frame {
                cands.push((m.last_used, true, i));
            }
        }
        for (i, t) in self.txds.iter().enumerate() {
            if matches!(t.state, Slot::Ready(_)) && t.last_used + 30 < self.frame {
                cands.push((t.last_used, false, i));
            }
        }
        cands.sort();
        for (_, is_model, i) in cands {
            if self.gpu_bytes <= target {
                break;
            }
            if is_model {
                if let Slot::Ready(m) = &self.models[i].state {
                    self.gpu_bytes -= m.bytes;
                }
                self.models[i].state = Slot::None;
            } else {
                self.gpu_bytes -= self.txds[i].bytes;
                self.txds[i].state = Slot::None;
                self.txds[i].bytes = 0;
            }
            self.stats.evicted += 1;
        }
    }

    /// Visibility over the LOD hierarchy (see module docs). Returns (instance, is_lod) to draw
    /// and the (distance, model) load requests.
    fn visibility(&self) -> (Vec<DrawEntry>, Vec<(f32, usize)>) {
        let cam = self.cam.pos;
        let hour = self.hour();
        let scale = self.opts.draw_dist_scale;
        let far = self.tc.colours(self.weather, self.minutes / 60.0).far_clip.max(300.0);
        let w = &self.world;
        let mut draw = Vec::new();
        let mut req = Vec::new();
        let visible_def = |i: usize| {
            let inst = &w.instances[i];
            let d = &w.models[inst.model];
            (inst.area == self.opts.area || inst.area == 13)
                && match d.kind {
                    ModelKind::Timed { on, off } => time_in_range(hour, on, off),
                    _ => true,
                }
        };
        let in_range = |i: usize| {
            let inst = &w.instances[i];
            let d = &w.models[inst.model];
            let dist = dist3(inst.pos, cam);
            (dist < d.draw_distance * scale && dist < far + 300.0, dist)
        };
        let mut stack: Vec<usize> = self.roots.clone();
        while let Some(i) = stack.pop() {
            if !visible_def(i) {
                continue;
            }
            let (ok, dist) = in_range(i);
            let children: Vec<usize> = w.instances[i]
                .lod_children
                .iter()
                .copied()
                .filter(|&c| visible_def(c) && (in_range(c).0 || w.instances[c].lod_children.iter().any(|&g| in_range(g).0)))
                .collect();
            let is_lod = !w.instances[i].lod_children.is_empty();
            if children.is_empty() {
                if ok && w.models[w.instances[i].model].draw_distance >= 2.0 {
                    draw.push((i, is_lod, dist));
                    req.push((dist, w.instances[i].model));
                }
                continue;
            }
            let all_ready = children.iter().all(|&c| self.model_ready(c));
            for &c in &children {
                req.push((dist3(w.instances[c].pos, cam), w.instances[c].model));
            }
            if all_ready {
                stack.extend(children);
            } else if ok {
                draw.push((i, true, dist));
                req.push((dist, w.instances[i].model));
            }
        }
        (draw, req)
    }

    /// One update step: streaming requests and uploads. `sync` loads everything visible before
    /// returning (headless screenshots).
    pub fn update(&mut self, sync: bool) {
        loop {
            let (_, mut req) = self.visibility();
            req.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut asked = 0;
            for (_, m) in req {
                if self.loader.in_flight >= self.opts.max_in_flight && !sync {
                    break;
                }
                if self.request_model(m) {
                    asked += 1;
                }
            }
            if !sync {
                self.pump(false);
                break;
            }
            // wait for everything in flight, then re-run visibility (LOD children)
            while self.loader.in_flight > 0 || !self.parked.is_empty() {
                if let Some(l) = self.loader.poll(true) {
                    match l {
                        Loaded::Txd { txd, result } => match result {
                            Ok(d) => self.upload_txd(txd, &d),
                            Err(_) => self.txds[txd].state = Slot::Failed,
                        },
                        Loaded::Model { model, result } => match result {
                            Ok(c) => self.parked.push((model, c)),
                            Err(_) => self.models[model].state = Slot::Failed,
                        },
                    }
                }
                self.pump(true);
            }
            if asked == 0 {
                break;
            }
        }
        self.evict();
    }

    /// Frame parameters for the current camera/time.
    pub fn frame_params(&self, aspect: f32) -> FrameParams {
        let c = self.tc.colours(self.weather, self.minutes / 60.0);
        let dir = self.cam.dir();
        FrameParams {
            view: sa_rw_wgpu::look_at(self.cam.pos, dir, [0.0, 0.0, 1.0]),
            fov_y: self.cam.fov_y,
            aspect,
            near: 0.3,
            cam_pos: self.cam.pos,
            fog_color: c.sky_bottom,
            fog_start: c.fog_start,
            fog_end: c.far_clip,
            dn_balance: dn_balance(self.minutes),
            ambient: c.amb_obj,
            dir_color: c.dir,
            light_dir: [-0.4, 0.3, -0.85],
            sky_top: c.sky_top,
            sky_bottom: c.sky_bottom,
            ambient_world: c.amb,
            color_filter: [0, 1, 2].map(|k| c.postfx1[k] * c.postfx1[3] + c.postfx2[k] * c.postfx2[3]),
        }
    }

    /// Builds the draw list and records the world pass.
    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, depth: &wgpu::TextureView, size: (u32, u32)) {
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        self.frame += 1;
        let p = self.frame_params(aspect);
        let colours = self.tc.colours(self.weather, self.minutes / 60.0);
        let (draw, _) = self.visibility();
        // frustum planes from proj * view (with a finite far plane for culling)
        let planes = frustum_planes(&p, colours.far_clip.max(300.0) + 200.0);
        let mut keep: Vec<(usize, Arc<GpuModel>, bool, f32)> = Vec::new();
        let frame = self.frame;
        for (i, is_lod, dist) in draw {
            let inst = &self.world.instances[i];
            let mi = inst.model;
            let Slot::Ready(m) = &self.models[mi].state else { continue };
            let ctr = [inst.pos[0], inst.pos[1], inst.pos[2]];
            let r = m.sphere[3] * max_scale(&inst.transform);
            if planes.iter().any(|pl| pl[0] * ctr[0] + pl[1] * ctr[1] + pl[2] * ctr[2] + pl[3] < -r) {
                continue;
            }
            keep.push((i, m.clone(), is_lod, dist));
            self.models[mi].last_used = frame;
            let chain = self.txd_chain(self.world.models[mi].txd);
            for t in chain {
                self.txds[t].last_used = frame;
            }
        }
        let mut items: Vec<DrawItem<'_>> = Vec::new();
        for (i, m, is_lod, dist) in &keep {
            let inst = &self.world.instances[*i];
            let def = &self.world.models[inst.model];
            // CRenderer::RenderEverythingBarRoads: alpha ref 140 outdoors; sorted list
            // (CVisibilityPlugins::RenderEntity): 100, or 0 without z writes.
            let pass = if def.draw_last() {
                Pass::Sorted { additive: def.additive(), zwrite: !def.no_zwrite() }
            } else {
                Pass::Main
            };
            let alpha_ref = match pass {
                Pass::Main => {
                    if self.opts.area == 0 {
                        140.0
                    } else {
                        0.0
                    }
                }
                Pass::Sorted { zwrite, .. } => {
                    if zwrite {
                        100.0
                    } else {
                        0.0
                    }
                }
            };
            for (local, g) in &m.parts {
                items.push(DrawItem {
                    geometry: g,
                    transform: mat_mul(&inst.transform, local),
                    pass,
                    cull: def.backface_culled(),
                    alpha_ref,
                    z_shift: if *is_lod { 100.0 } else { 0.0 },
                    dist: *dist,
                    alpha: 1.0,
                });
            }
        }
        if let Some(wg) = &self.water {
            self.renderer.set_mesh_color(wg, 0, colours.water);
            items.push(DrawItem {
                geometry: wg,
                transform: RwMatrix::IDENTITY.to_cols(),
                pass: Pass::Sorted { additive: false, zwrite: false },
                cull: false,
                alpha_ref: 0.0,
                z_shift: 0.0,
                dist: f32::MAX,
                alpha: 1.0,
            });
        }
        self.renderer.render(encoder, target, depth, size, &p, &items);
        let rs = self.renderer.stats;
        self.stats = ViewerStats {
            visible: keep.len(),
            models_loaded: self.models.iter().filter(|m| matches!(m.state, Slot::Ready(_))).count(),
            txds_loaded: self.txds.iter().filter(|m| matches!(m.state, Slot::Ready(_))).count(),
            pending: self.loader.in_flight + self.parked.len(),
            gpu_mb: self.gpu_bytes as f64 / (1 << 20) as f64,
            draw_calls: rs.draw_calls,
            triangles: rs.triangles,
            evicted: self.stats.evicted,
        };
    }

    pub fn loader_bytes_read(&self) -> u64 {
        self.loader.bytes_read
    }
}

/// (instance, is LOD, distance) of an entity to draw.
type DrawEntry = (usize, bool, f32);

/// Six planes (a, b, c, d) with inside = a*x + b*y + c*z + d >= 0.
fn frustum_planes(p: &FrameParams, far: f32) -> [[f32; 4]; 6] {
    let f = 1.0 / (p.fov_y * 0.5).tan();
    let proj = [[f / p.aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, -far / (far - p.near), -1.0], [
        0.0,
        0.0,
        -far * p.near / (far - p.near),
        0.0,
    ]];
    let m = mat_mul(&proj, &p.view);
    let row = |r: usize| [m[0][r], m[1][r], m[2][r], m[3][r]];
    let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
    let add = |a: [f32; 4], b: [f32; 4], s: f32| [a[0] + s * b[0], a[1] + s * b[1], a[2] + s * b[2], a[3] + s * b[3]];
    let norm = |pl: [f32; 4]| {
        let l = (pl[0] * pl[0] + pl[1] * pl[1] + pl[2] * pl[2]).sqrt().max(1e-12);
        [pl[0] / l, pl[1] / l, pl[2] / l, pl[3] / l]
    };
    [
        norm(add(r3, r0, 1.0)),
        norm(add(r3, r0, -1.0)),
        norm(add(r3, r1, 1.0)),
        norm(add(r3, r1, -1.0)),
        norm(r2),
        norm(add(r3, r2, -1.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_range_wraps() {
        assert!(time_in_range(22, 20, 6));
        assert!(time_in_range(3, 20, 6));
        assert!(!time_in_range(12, 20, 6));
        assert!(time_in_range(12, 6, 20));
        assert!(!time_in_range(20, 6, 20));
    }

    #[test]
    fn camera_yaw_zero_looks_north() {
        let mut c = Camera { pos: [0.0; 3], yaw: 0.0, pitch: 0.0, fov_y: 1.0 };
        let d = c.dir();
        assert!(d[1] > 0.99);
        c.look_dir([1.0, 0.0, 0.0]);
        assert!((c.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!(c.right()[1].abs() < 1e-6 && c.right()[0].abs() < 1e-6 || c.right()[1] < -0.99);
    }

    #[test]
    fn frustum_contains_point_ahead() {
        let mut c = Camera { pos: [0.0; 3], yaw: 0.0, pitch: 0.0, fov_y: 1.0 };
        c.look_dir([0.0, 1.0, 0.0]);
        let p = FrameParams {
            view: sa_rw_wgpu::look_at(c.pos, c.dir(), [0.0, 0.0, 1.0]),
            fov_y: 1.0,
            aspect: 1.5,
            near: 0.3,
            cam_pos: c.pos,
            fog_color: [0.0; 3],
            fog_start: 0.0,
            fog_end: 1.0,
            dn_balance: 0.0,
            ambient: [0.0; 3],
            dir_color: [0.0; 3],
            light_dir: [0.0; 3],
            sky_top: [0.0; 3],
            sky_bottom: [0.0; 3],
            ambient_world: [0.0; 3],
            color_filter: [0.0; 3],
        };
        let pl = frustum_planes(&p, 1000.0);
        let inside = |q: [f32; 3]| pl.iter().all(|p| p[0] * q[0] + p[1] * q[1] + p[2] * q[2] + p[3] >= 0.0);
        assert!(inside([0.0, 10.0, 0.0]));
        assert!(!inside([0.0, -10.0, 0.0]));
        assert!(!inside([0.0, 2000.0, 0.0]));
        assert!(!inside([100.0, 10.0, 0.0]));
    }
}
