//! The static world: `default.dat`/`gta.dat` → IMG archives, IDE model definitions, text and
//! binary IPL instances with their LOD links, `water.dat`.
//!
//! Parsing is `sa-formats` (the exe's own line reader and `sscanf` behaviour, FMT-2). The
//! derivations follow the original loaders (gta-reversed as reading aid, not measured yet):
//! * model flags: `CBaseModelInfo::SetBaseModelInfoFlags` and
//!   `CAtomicModelInfo::SetAtomicModelInfoFlags` — the ported, L1-verified pure helpers
//!   `sa_stream::modelinfo::{base, atomic}` are called directly;
//! * instance matrices: `CFileLoader::LoadObjectInstance` (0x538090; quaternion branch through
//!   the ported `sa_math::CQuaternion::get`, heading branch through the ported
//!   `CMatrix::set_rotate_z_only`; `acos` is Rust's, the original's `_CIacos` is not ported);
//! * LOD links: text IPL `lod` = index in the same file; binary `<name>_streamN.ipl` `lod` =
//!   index in the related text IPL (`CIplStore::SetupRelatedIpls`: name prefix `<name>_stream`).

use std::collections::HashMap;
use std::sync::Arc;

use sa_formats::data::gta_dat::{Directive, LevelFile};
use sa_formats::data::ide::Ide;
use sa_formats::data::ipl::Ipl;
use sa_formats::data::ipl_bin::BinaryIpl;
use sa_formats::data::world::water::{Shape, Water};
use sa_formats::img::{ImgDirectory, ImgEntry};
use sa_math::matrix::CMatrix;
use sa_math::quaternion::CQuaternion;
use sa_math::vector::CVector;
use sa_platform::fs::GameFs;
use sa_stream::modelinfo::{atomic::atomic_flags_word, base::base_model_info_flags_byte};

/// One archive (`CStreaming::ms_files`).
pub struct ImgArchive {
    pub path: String,
    pub dir: ImgDirectory,
}

/// Location of an archive entry.
#[derive(Debug, Clone, Copy)]
pub struct EntryRef {
    pub img: usize,
    pub offset: u64,
    pub len: usize,
}

impl EntryRef {
    fn new(img: usize, e: &ImgEntry) -> EntryRef {
        EntryRef { img, offset: e.byte_offset() as u64, len: e.byte_len() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// `objs` (`CAtomicModelInfo`)
    Atomic,
    /// `tobj` (`CTimeModelInfo`): visible between the hours `on` and `off`.
    Timed { on: u8, off: u8 },
    /// `hier` / `anim` (`CClumpModelInfo`)
    Clump,
}

/// A model definition (`CBaseModelInfo` fields this viewer needs).
#[derive(Debug, Clone)]
pub struct ModelDef {
    /// Lower-case model name (DFF without extension).
    pub name: String,
    /// Index into [`World::txds`].
    pub txd: Option<usize>,
    pub draw_distance: f32,
    pub kind: ModelKind,
    /// `CBaseModelInfo` flag byte (+0x12) and word (+0x12..0x14), as the ported setters compute
    /// them from the IDE flags.
    pub flag_byte: u8,
    pub flag_word: u16,
    pub dff: Option<EntryRef>,
}

impl ModelDef {
    pub fn draw_last(&self) -> bool {
        self.flag_byte & 0x02 != 0
    }
    pub fn additive(&self) -> bool {
        self.flag_byte & 0x04 != 0
    }
    pub fn no_zwrite(&self) -> bool {
        self.flag_byte & 0x08 != 0
    }
    pub fn backface_culled(&self) -> bool {
        self.flag_byte & 0x40 != 0
    }
    /// `CBaseModelInfo::IsRoad` (word bit 0x0100 after `SetAtomicModelInfoFlags`); the road
    /// pass (`CRenderer::RenderRoads`) is not separated yet.
    #[allow(dead_code)]
    pub fn is_road(&self) -> bool {
        self.flag_word & 0x0100 != 0
    }
}

/// A texture dictionary slot (`CTxdStore`).
#[derive(Debug, Clone)]
pub struct TxdDef {
    pub name: String,
    pub parent: Option<usize>,
    pub entry: Option<EntryRef>,
}

/// A placed building/dummy (`CFileObjectInstance` after `LoadObjectInstance`).
#[derive(Debug, Clone)]
pub struct Instance {
    pub model: usize,
    /// Model → world (column-major).
    pub transform: [[f32; 4]; 4],
    pub pos: [f32; 3],
    pub area: u8,
    pub lod_parent: Option<usize>,
    pub lod_children: Vec<usize>,
}

/// (model id, position, rotation quaternion, instance flags, LOD index) of one IPL line.
type InstRec = (i32, [f32; 3], [f32; 4], u32, i32);

/// A water polygon (`water.dat`).
#[derive(Debug, Clone)]
pub struct WaterPoly {
    pub verts: Vec<[f32; 3]>,
}

pub struct World {
    pub archives: Vec<ImgArchive>,
    pub models: Vec<ModelDef>,
    pub model_by_id: HashMap<i32, usize>,
    pub txds: Vec<TxdDef>,
    pub instances: Vec<Instance>,
    pub water: Vec<WaterPoly>,
    pub timecyc: Vec<u8>,
    pub warnings: Vec<String>,
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).trim().to_string()
}

fn read_img_dir(fs: &dyn GameFs, path: &str) -> std::io::Result<ImgDirectory> {
    let mut f = fs.open(path)?;
    let mut head = [0u8; 8];
    f.read_at(0, &mut head)?;
    let n = ImgDirectory::header_len_from_prefix(&head).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut buf = vec![0u8; n];
    f.read_at(0, &mut buf)?;
    ImgDirectory::parse(&buf).map_err(|e| std::io::Error::other(e.to_string()))
}

/// `CFileLoader::LoadObjectInstance` matrix setup (see module docs).
pub fn instance_matrix(pos: [f32; 3], q: [f32; 4], flags: u32) -> [[f32; 4]; 4] {
    let dont_stream = flags & 0x200 != 0;
    let mut m = CMatrix::default();
    if q[0].abs() > 0.05 || q[1].abs() > 0.05 || (dont_stream && q[0] != 0.0 && q[1] != 0.0) {
        let quat = CQuaternion { x: -q[0], y: -q[1], z: -q[2], w: q[3] };
        let mut rw = m.rw();
        quat.get(&mut rw);
        m.right = rw.right;
        m.forward = rw.forward;
        m.up = rw.up;
    } else {
        let mult = if q[2] < 0.0 { 2.0 } else { -2.0 };
        let heading = q[3].clamp(-1.0, 1.0).acos() * mult;
        m.set_rotate_z_only(heading);
    }
    let c = |v: CVector, w: f32| [v.x, v.y, v.z, w];
    [c(m.right, 0.0), c(m.forward, 0.0), c(m.up, 0.0), [pos[0], pos[1], pos[2], 1.0]]
}

impl World {
    pub fn load(fs: &dyn GameFs) -> Result<World, String> {
        let mut w = World {
            archives: Vec::new(),
            models: Vec::new(),
            model_by_id: HashMap::new(),
            txds: Vec::new(),
            instances: Vec::new(),
            water: Vec::new(),
            timecyc: Vec::new(),
            warnings: Vec::new(),
        };
        // CStreaming::Init adds these two before any level file.
        for p in ["MODELS\\GTA3.IMG", "MODELS\\GTA_INT.IMG"] {
            match read_img_dir(fs, p) {
                Ok(dir) => w.archives.push(ImgArchive { path: p.to_string(), dir }),
                Err(e) => return Err(format!("{p}: {e}")),
            }
        }
        let mut ipl_paths = Vec::new();
        let mut ide_paths = Vec::new();
        for level in ["DATA\\DEFAULT.DAT", "DATA\\GTA.DAT"] {
            let data = fs.read(level).map_err(|e| format!("{level}: {e}"))?;
            for e in LevelFile::parse(&data).entries {
                let path = lossy(e.path());
                match e.directive {
                    Directive::Ide => ide_paths.push(path),
                    Directive::Ipl => ipl_paths.push(path),
                    Directive::Img if e.img_added => match read_img_dir(fs, &path) {
                        Ok(dir) => w.archives.push(ImgArchive { path, dir }),
                        Err(err) => w.warnings.push(format!("{path}: {err}")),
                    },
                    _ => {}
                }
            }
        }
        let mut txd_by_name: HashMap<String, usize> = HashMap::new();
        let mut txd_slot = |w: &mut World, name: &str| -> usize {
            let name = name.to_ascii_lowercase();
            *txd_by_name.entry(name.clone()).or_insert_with(|| {
                w.txds.push(TxdDef { name, parent: None, entry: None });
                w.txds.len() - 1
            })
        };
        let mut parents = Vec::new();
        for p in &ide_paths {
            let data = match fs.read(p) {
                Ok(d) => d,
                Err(e) => {
                    w.warnings.push(format!("{p}: {e}"));
                    continue;
                }
            };
            let ide = Ide::parse(&data);
            let mut add = |w: &mut World, id: Option<i32>, model: &Option<Vec<u8>>, txd: &Option<Vec<u8>>, dd: f32, flags: u32, kind| {
                let (Some(id), Some(model)) = (id, model) else { return };
                let t = txd.as_ref().map(|t| txd_slot(w, &lossy(t)));
                let flag_byte = base_model_info_flags_byte(0, flags);
                let flag_word = if kind == ModelKind::Clump {
                    flag_byte as u16
                } else {
                    atomic_flags_word(flag_byte as u16, flags)
                };
                w.model_by_id.insert(id, w.models.len());
                w.models.push(ModelDef {
                    name: lossy(model).to_ascii_lowercase(),
                    txd: t,
                    draw_distance: dd,
                    kind,
                    flag_byte,
                    flag_word,
                    dff: None,
                });
            };
            for o in &ide.objs {
                add(&mut w, o.id, &o.model, &o.txd, o.draw_distance.unwrap_or(0.0), o.flags.unwrap_or(0), ModelKind::Atomic);
            }
            for o in &ide.tobj {
                let kind = ModelKind::Timed { on: o.time_on.unwrap_or(0) as u8, off: o.time_off.unwrap_or(24) as u8 };
                add(&mut w, o.id, &o.model, &o.txd, o.draw_distance.unwrap_or(0.0), o.flags.unwrap_or(0), kind);
            }
            for o in &ide.anim {
                add(&mut w, Some(o.id), &o.model, &o.txd, o.draw_distance, o.flags.unwrap_or(0), ModelKind::Clump);
            }
            for o in &ide.hier {
                add(&mut w, Some(o.id), &o.model, &o.txd, 2000.0, 0, ModelKind::Clump);
            }
            for t in &ide.txdp {
                if let (Some(a), Some(b)) = (&t.txd, &t.parent) {
                    parents.push((lossy(a), lossy(b)));
                }
            }
        }
        for (a, b) in parents {
            let (ia, ib) = (txd_slot(&mut w, &a), txd_slot(&mut w, &b));
            w.txds[ia].parent = Some(ib);
        }
        // IMG entries: first archive with the name wins.
        let mut dff_by_name: HashMap<String, EntryRef> = HashMap::new();
        let mut txd_entries: HashMap<String, EntryRef> = HashMap::new();
        let mut bin_ipls: Vec<(String, EntryRef)> = Vec::new();
        for (ai, a) in w.archives.iter().enumerate() {
            for e in &a.dir.entries {
                let n = e.name_lossy().to_ascii_lowercase();
                if let Some(s) = n.strip_suffix(".dff") {
                    dff_by_name.entry(s.to_string()).or_insert(EntryRef::new(ai, e));
                } else if let Some(s) = n.strip_suffix(".txd") {
                    txd_entries.entry(s.to_string()).or_insert(EntryRef::new(ai, e));
                } else if let Some(s) = n.strip_suffix(".ipl") {
                    bin_ipls.push((s.to_string(), EntryRef::new(ai, e)));
                }
            }
        }
        for m in &mut w.models {
            m.dff = dff_by_name.get(&m.name).copied();
        }
        for t in &mut w.txds {
            t.entry = txd_entries.get(&t.name).copied();
        }

        // Text IPLs (gta.dat order), then the streamed binary ones linked to them.
        let mut text_bases: Vec<(String, usize)> = Vec::new();
        for p in &ipl_paths {
            let data = match fs.read(p) {
                Ok(d) => d,
                Err(e) => {
                    w.warnings.push(format!("{p}: {e}"));
                    continue;
                }
            };
            let ipl = Ipl::parse(&data);
            let base = w.instances.len();
            let stem = p.rsplit(['\\', '/']).next().unwrap_or(p).split('.').next().unwrap_or("").to_ascii_lowercase();
            text_bases.push((stem, base));
            let recs: Vec<_> = ipl
                .inst
                .iter()
                .map(|i| {
                    let pos = [i.pos[0].unwrap_or(0.0), i.pos[1].unwrap_or(0.0), i.pos[2].unwrap_or(0.0)];
                    let rot = [i.rot[0].unwrap_or(0.0), i.rot[1].unwrap_or(0.0), i.rot[2].unwrap_or(0.0), i.rot[3].unwrap_or(1.0)];
                    (i.id.unwrap_or(-1), pos, rot, i.interior.unwrap_or(0) as u32, i.lod.unwrap_or(-1))
                })
                .collect();
            w.add_instances(&recs, base);
        }
        let mut stream_count = 0;
        for (name, er) in bin_ipls {
            let Some(_stream_pos) = name.find("_stream") else { continue };
            let mut buf = vec![0u8; er.len];
            let mut f = fs.open(&w.archives[er.img].path).map_err(|e| e.to_string())?;
            if f.read_at(er.offset, &mut buf).is_err() {
                continue;
            }
            let Ok(b) = BinaryIpl::parse(&buf) else {
                w.warnings.push(format!("{name}.ipl: not a binary IPL"));
                continue;
            };
            // CIplStore::SetupRelatedIpls: "<text stem>_stream" is a prefix of the entry name.
            let base = text_bases
                .iter()
                .find(|(stem, _)| name.starts_with(&format!("{stem}_stream")))
                .map(|(_, b)| *b);
            let recs: Vec<_> = b.inst.iter().map(|i| (i.model_id, i.pos, i.rot, i.instance_type, i.lod)).collect();
            match base {
                Some(base) => w.add_instances(&recs, base),
                None => {
                    w.warnings.push(format!("{name}.ipl: no related text IPL"));
                    w.add_instances(&recs, usize::MAX)
                }
            }
            stream_count += 1;
        }
        if stream_count == 0 {
            w.warnings.push("no streamed binary IPLs found in the archives".into());
        }

        // water.dat
        if let Ok(data) = fs.read("DATA\\water.dat") {
            for e in Water::parse(&data).entries {
                let n = if e.shape == Shape::Triangle { 3 } else { 4 };
                let verts: Vec<[f32; 3]> = (0..n)
                    .map(|k| {
                        let v = e.vertex(k);
                        [v[0].unwrap_or(0.0), v[1].unwrap_or(0.0), v[2].unwrap_or(0.0)]
                    })
                    .collect();
                w.water.push(WaterPoly { verts });
            }
        }
        w.timecyc = fs.read("DATA\\timecyc.dat").map_err(|e| format!("timecyc.dat: {e}"))?;
        Ok(w)
    }

    /// Adds instances; `lod` indices are relative to `lod_base` (`usize::MAX` = no LOD links).
    fn add_instances(&mut self, recs: &[InstRec], lod_base: usize) {
        let first = self.instances.len();
        let mut pending = Vec::new();
        for &(id, pos, rot, flags, lod) in recs {
            let Some(&model) = self.model_by_id.get(&id) else {
                // keep indices stable for LOD links of later instances of this file
                self.instances.push(Instance {
                    model: usize::MAX,
                    transform: [[0.0; 4]; 4],
                    pos,
                    area: 255,
                    lod_parent: None,
                    lod_children: Vec::new(),
                });
                continue;
            };
            let idx = self.instances.len();
            self.instances.push(Instance {
                model,
                transform: instance_matrix(pos, rot, flags),
                pos,
                area: (flags & 0xFF) as u8,
                lod_parent: None,
                lod_children: Vec::new(),
            });
            if lod >= 0 && lod_base != usize::MAX {
                pending.push((idx, lod_base + lod as usize));
            }
        }
        let _ = first;
        for (child, parent) in pending {
            if parent < self.instances.len() && parent != child && self.instances[parent].model != usize::MAX {
                self.instances[child].lod_parent = Some(parent);
                self.instances[parent].lod_children.push(child);
            }
        }
    }

    /// Opens every archive for the loader thread.
    pub fn archive_paths(&self) -> Arc<Vec<String>> {
        Arc::new(self.archives.iter().map(|a| a.path.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_branch_matches_quaternion_branch() {
        // a pure z rotation of 90 degrees: q = (0, 0, sin 45, cos 45)
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let m = instance_matrix([1.0, 2.0, 3.0], [0.0, 0.0, s, s], 0);
        assert_eq!(m[3], [1.0, 2.0, 3.0, 1.0]);
        // heading = acos(w) * -2 = -90 degrees: the conjugated rotation, as the quaternion
        // branch computes it
        let mut rw = CMatrix::default().rw();
        CQuaternion { x: 0.0, y: 0.0, z: -s, w: s }.get(&mut rw);
        let q = [[rw.right.x, rw.right.y, rw.right.z], [rw.forward.x, rw.forward.y, rw.forward.z]];
        for c in 0..2 {
            for r in 0..3 {
                assert!((m[c][r] - q[c][r]).abs() < 1e-5, "{m:?} vs {q:?}");
            }
        }
        assert!((m[0][1] + 1.0).abs() < 1e-5);
    }
}
