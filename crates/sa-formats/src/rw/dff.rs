//! Typed decoding of DFF model files (RpClump streams).
//!
//! Struct layouts follow the RenderWare 3.x stream format as documented by gtamods ("RenderWare
//! binary stream file") and implemented by librw (`clump.cpp`, `geometry.cpp`); they are verified
//! by the byte-exact typed round trip over every DFF in the 1.0 US install (see
//! `tests/game_data.rs`). Field meanings that are not confirmed against `gta_sa.exe` yet are noted
//! in `docs/re/formats.md`.
//!
//! Every node stores the version stamp of its own chunk and of its Struct chunk, so
//! `to_chunk` rebuilds the identical tree.

use super::chunk::Children;
use super::{Chunk, Extension, LibraryStamp, RwString, id};
use crate::bytes::{Put, Reader};
use crate::error::{Error, Result};

/// `rpGEOMETRY*` format flags (low byte of the geometry format word).
pub mod geometry_flags {
    pub const TRISTRIP: u32 = 0x01;
    pub const POSITIONS: u32 = 0x02;
    pub const TEXTURED: u32 = 0x04;
    pub const PRELIT: u32 = 0x08;
    pub const NORMALS: u32 = 0x10;
    pub const LIGHT: u32 = 0x20;
    pub const MODULATE_MATERIAL_COLOR: u32 = 0x40;
    pub const TEXTURED2: u32 = 0x80;
    /// `rpGEOMETRYNATIVE`: vertex data is in a platform-native plugin, not in the struct.
    pub const NATIVE: u32 = 0x0100_0000;
}

/// A top-level item of a DFF file.
#[derive(Debug, Clone, PartialEq)]
pub enum DffItem {
    Clump(Box<Clump>),
    /// Any other top-level chunk (e.g. the UV animation dictionary, 0x2B), kept raw.
    Other(Chunk),
}

impl DffItem {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        Ok(if c.ty == id::CLUMP {
            DffItem::Clump(Box::new(Clump::from_chunk(c)?))
        } else {
            DffItem::Other(c.clone())
        })
    }

    pub fn to_chunk(&self) -> Result<Chunk> {
        match self {
            DffItem::Clump(c) => c.to_chunk(),
            DffItem::Other(c) => Ok(c.clone()),
        }
    }
}

/// A light or camera attached to a clump: frame index struct + the object chunk (kept raw).
#[derive(Debug, Clone, PartialEq)]
pub struct ClumpObject {
    pub struct_version: u32,
    pub frame_index: i32,
    pub chunk: Chunk,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clump {
    pub version: u32,
    pub struct_version: u32,
    /// Whether the struct carries light/camera counts (library version > 0x33000).
    pub has_light_camera_counts: bool,
    pub frame_list: FrameList,
    pub geometry_list: GeometryList,
    pub atomics: Vec<Atomic>,
    pub lights: Vec<ClumpObject>,
    pub cameras: Vec<ClumpObject>,
    pub extension: Extension,
}

impl Clump {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW clump";
        let mut ch = Children::of(c, id::CLUMP, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let num_atomics = r.i32()?;
        let has_counts = LibraryStamp(s.version).version() > 0x33000;
        let (num_lights, num_cameras) = if has_counts { (r.i32()?, r.i32()?) } else { (0, 0) };
        r.expect_end()?;
        let frame_list = FrameList::from_chunk(ch.next(id::FRAMELIST)?)?;
        let geometry_list = GeometryList::from_chunk(ch.next(id::GEOMETRYLIST)?)?;
        let mut atomics = Vec::new();
        for _ in 0..num_atomics {
            atomics.push(Atomic::from_chunk(ch.next(id::ATOMIC)?)?);
        }
        let mut objects = |n: i32, ty: u32| -> Result<Vec<ClumpObject>> {
            let mut v = Vec::new();
            for _ in 0..n {
                let s = ch.next(id::STRUCT)?;
                let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
                let frame_index = r.i32()?;
                r.expect_end()?;
                v.push(ClumpObject {
                    struct_version: s.version,
                    frame_index,
                    chunk: ch.next(ty)?.clone(),
                });
            }
            Ok(v)
        };
        let lights = objects(num_lights, id::LIGHT)?;
        let cameras = objects(num_cameras, id::CAMERA)?;
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(Clump {
            version: c.version,
            struct_version: s.version,
            has_light_camera_counts: has_counts,
            frame_list,
            geometry_list,
            atomics,
            lights,
            cameras,
            extension,
        })
    }

    pub fn to_chunk(&self) -> Result<Chunk> {
        let mut s = Vec::new();
        s.put_i32(self.atomics.len() as i32);
        if self.has_light_camera_counts {
            s.put_i32(self.lights.len() as i32);
            s.put_i32(self.cameras.len() as i32);
        }
        let mut children = vec![
            Chunk::new_data(id::STRUCT, self.struct_version, s),
            self.frame_list.to_chunk(),
            self.geometry_list.to_chunk()?,
        ];
        for a in &self.atomics {
            children.push(a.to_chunk()?);
        }
        for o in self.lights.iter().chain(&self.cameras) {
            children.push(Chunk::new_data(
                id::STRUCT,
                o.struct_version,
                o.frame_index.to_le_bytes().to_vec(),
            ));
            children.push(o.chunk.clone());
        }
        children.push(self.extension.to_chunk());
        Ok(Chunk::new_children(id::CLUMP, self.version, children))
    }
}

/// One frame of a frame list (56 bytes in the struct).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Rotation matrix rows: right, up, at.
    pub rotation: [[f32; 3]; 3],
    pub position: [f32; 3],
    /// Parent frame index, -1 for the root.
    pub parent: i32,
    /// Matrix flags word (`RwMatrix::flags`); kept raw.
    pub matrix_flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameList {
    pub version: u32,
    pub struct_version: u32,
    pub frames: Vec<Frame>,
    /// One extension per frame (holds node names, HAnim data).
    pub extensions: Vec<Extension>,
}

impl FrameList {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW frame list";
        let mut ch = Children::of(c, id::FRAMELIST, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let n = r.u32()? as usize;
        r.check_count(n, 56)?;
        let mut frames = Vec::with_capacity(n);
        for _ in 0..n {
            frames.push(Frame {
                rotation: [r.f32s()?, r.f32s()?, r.f32s()?],
                position: r.f32s()?,
                parent: r.i32()?,
                matrix_flags: r.u32()?,
            });
        }
        r.expect_end()?;
        let mut extensions = Vec::with_capacity(n);
        for _ in 0..n {
            extensions.push(Extension::next(&mut ch)?);
        }
        ch.finish()?;
        Ok(FrameList {
            version: c.version,
            struct_version: s.version,
            frames,
            extensions,
        })
    }

    pub fn to_chunk(&self) -> Chunk {
        let mut s = Vec::new();
        s.put_u32(self.frames.len() as u32);
        for f in &self.frames {
            for row in &f.rotation {
                s.put_f32s(row);
            }
            s.put_f32s(&f.position);
            s.put_i32(f.parent);
            s.put_u32(f.matrix_flags);
        }
        let mut children = vec![Chunk::new_data(id::STRUCT, self.struct_version, s)];
        children.extend(self.extensions.iter().map(Extension::to_chunk));
        Chunk::new_children(id::FRAMELIST, self.version, children)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryList {
    pub version: u32,
    pub struct_version: u32,
    pub geometries: Vec<Geometry>,
}

impl GeometryList {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW geometry list";
        let mut ch = Children::of(c, id::GEOMETRYLIST, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let n = r.u32()?;
        r.expect_end()?;
        let mut geometries = Vec::new();
        for _ in 0..n {
            geometries.push(Geometry::from_chunk(ch.next(id::GEOMETRY)?)?);
        }
        ch.finish()?;
        Ok(GeometryList {
            version: c.version,
            struct_version: s.version,
            geometries,
        })
    }

    pub fn to_chunk(&self) -> Result<Chunk> {
        let count = (self.geometries.len() as u32).to_le_bytes().to_vec();
        let mut children = vec![Chunk::new_data(id::STRUCT, self.struct_version, count)];
        for g in &self.geometries {
            children.push(g.to_chunk()?);
        }
        Ok(Chunk::new_children(id::GEOMETRYLIST, self.version, children))
    }
}

/// A triangle. In the stream the four u16 are stored as `v[1], v[0], material, v[2]`
/// (librw `Geometry::streamRead` reads two u32: `v0 = w0 >> 16`, `v1 = w0 & 0xFFFF`,
/// `v2 = w1 >> 16`, `mat = w1 & 0xFFFF`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triangle {
    pub v: [u16; 3],
    pub material: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MorphTarget {
    /// Bounding sphere: centre x, y, z, radius.
    pub bounding_sphere: [f32; 4],
    /// Raw "has vertices" word; vertices are present iff non-zero.
    pub has_vertices: u32,
    /// Raw "has normals" word; normals are present iff non-zero.
    pub has_normals: u32,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

/// Decoded geometry struct.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryData {
    /// Full format word: flags in bits 0-7, texture coordinate set count in bits 16-23,
    /// `rpGEOMETRYNATIVE` in bit 24 (bits 8-15 and 25-31 kept as stored).
    pub format: u32,
    pub num_triangles: i32,
    pub num_vertices: i32,
    /// Ambient, specular, diffuse; present only for library versions < 0x34000.
    pub surface_props: Option<[f32; 3]>,
    /// Pre-lit vertex colours (RGBA), present if `PRELIT` and not native.
    pub prelit: Vec<[u8; 4]>,
    /// Texture coordinate sets, each `num_vertices` (u, v) pairs.
    pub tex_coords: Vec<Vec<[f32; 2]>>,
    pub triangles: Vec<Triangle>,
    pub morph_targets: Vec<MorphTarget>,
}

impl GeometryData {
    pub fn flags(&self) -> u32 {
        self.format & 0xFF
    }

    pub fn is_native(&self) -> bool {
        self.format & geometry_flags::NATIVE != 0
    }

    /// Number of texture coordinate sets as RW derives it: bits 16-23, or 2/1/0 from
    /// `TEXTURED2`/`TEXTURED` when that byte is zero.
    pub fn num_tex_sets(&self) -> usize {
        let n = ((self.format >> 16) & 0xFF) as usize;
        if n != 0 {
            n
        } else if self.format & geometry_flags::TEXTURED2 != 0 {
            2
        } else if self.format & geometry_flags::TEXTURED != 0 {
            1
        } else {
            0
        }
    }

    pub fn parse(data: &[u8], stamp: u32) -> Result<Self> {
        const CTX: &str = "RW geometry struct";
        let mut r = Reader::new(data, CTX);
        let format = r.u32()?;
        let num_triangles = r.i32()?;
        let num_vertices = r.i32()?;
        let num_morph = r.i32()?;
        if num_triangles < 0 || num_vertices < 0 || num_morph < 0 {
            return Err(Error::invalid(CTX, "negative count"));
        }
        let (nt, nv) = (num_triangles as usize, num_vertices as usize);
        let surface_props = if LibraryStamp(stamp).version() < 0x34000 {
            Some(r.f32s()?)
        } else {
            None
        };
        let mut g = GeometryData {
            format,
            num_triangles,
            num_vertices,
            surface_props,
            prelit: Vec::new(),
            tex_coords: Vec::new(),
            triangles: Vec::new(),
            morph_targets: Vec::new(),
        };
        if !g.is_native() {
            if format & geometry_flags::PRELIT != 0 {
                r.check_count(nv, 4)?;
                g.prelit = (0..nv).map(|_| r.array()).collect::<Result<_>>()?;
            }
            for _ in 0..g.num_tex_sets() {
                r.check_count(nv, 8)?;
                g.tex_coords
                    .push((0..nv).map(|_| r.f32s()).collect::<Result<_>>()?);
            }
            r.check_count(nt, 8)?;
            for _ in 0..nt {
                let (v1, v0, material, v2) = (r.u16()?, r.u16()?, r.u16()?, r.u16()?);
                g.triangles.push(Triangle {
                    v: [v0, v1, v2],
                    material,
                });
            }
        }
        for _ in 0..num_morph {
            let bounding_sphere = r.f32s()?;
            let has_vertices = r.u32()?;
            let has_normals = r.u32()?;
            let mut read_vecs = |present: bool| -> Result<Vec<[f32; 3]>> {
                if !present {
                    return Ok(Vec::new());
                }
                r.check_count(nv, 12)?;
                (0..nv).map(|_| r.f32s()).collect()
            };
            let vertices = read_vecs(has_vertices != 0)?;
            let normals = read_vecs(has_normals != 0)?;
            g.morph_targets.push(MorphTarget {
                bounding_sphere,
                has_vertices,
                has_normals,
                vertices,
                normals,
            });
        }
        r.expect_end()?;
        Ok(g)
    }

    /// Serialises the struct. Fails if array lengths disagree with the counts/flags.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        const CTX: &str = "RW geometry struct (write)";
        let (nt, nv) = (self.num_triangles as usize, self.num_vertices as usize);
        let bad = |what: &str| Err(Error::invalid(CTX, format!("{what} does not match the header")));
        let mut s = Vec::new();
        s.put_u32(self.format);
        s.put_i32(self.num_triangles);
        s.put_i32(self.num_vertices);
        s.put_i32(self.morph_targets.len() as i32);
        if let Some(p) = &self.surface_props {
            s.put_f32s(p);
        }
        if !self.is_native() {
            let prelit = self.format & geometry_flags::PRELIT != 0;
            if self.prelit.len() != if prelit { nv } else { 0 } {
                return bad("prelit colour count");
            }
            for c in &self.prelit {
                s.extend_from_slice(c);
            }
            if self.tex_coords.len() != self.num_tex_sets() || self.tex_coords.iter().any(|t| t.len() != nv) {
                return bad("texture coordinate sets");
            }
            for set in &self.tex_coords {
                for uv in set {
                    s.put_f32s(uv);
                }
            }
            if self.triangles.len() != nt {
                return bad("triangle count");
            }
            for t in &self.triangles {
                s.put_u16(t.v[1]);
                s.put_u16(t.v[0]);
                s.put_u16(t.material);
                s.put_u16(t.v[2]);
            }
        }
        for m in &self.morph_targets {
            s.put_f32s(&m.bounding_sphere);
            s.put_u32(m.has_vertices);
            s.put_u32(m.has_normals);
            for (present, list) in [
                (m.has_vertices != 0, &m.vertices),
                (m.has_normals != 0, &m.normals),
            ] {
                if list.len() != if present { nv } else { 0 } {
                    return bad("morph target vertex/normal count");
                }
                for v in list {
                    s.put_f32s(v);
                }
            }
        }
        Ok(s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub version: u32,
    pub struct_version: u32,
    pub data: GeometryData,
    pub material_list: MaterialList,
    /// Extension (bin mesh, skin, breakable, night colours, 2dfx, ...), raw.
    pub extension: Extension,
}

impl Geometry {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW geometry";
        let mut ch = Children::of(c, id::GEOMETRY, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let data = GeometryData::parse(s.expect_data(id::STRUCT, CTX)?, s.version)?;
        let material_list = MaterialList::from_chunk(ch.next(id::MATLIST)?)?;
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(Geometry {
            version: c.version,
            struct_version: s.version,
            data,
            material_list,
            extension,
        })
    }

    /// Fails if the geometry data is inconsistent with its counts (see [`GeometryData::to_bytes`]).
    pub fn to_chunk(&self) -> Result<Chunk> {
        Ok(Chunk::new_children(
            id::GEOMETRY,
            self.version,
            vec![
                Chunk::new_data(id::STRUCT, self.struct_version, self.data.to_bytes()?),
                self.material_list.to_chunk(),
                self.extension.to_chunk(),
            ],
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialList {
    pub version: u32,
    pub struct_version: u32,
    /// Per material slot: -1 = a new material (next entry of `materials`), otherwise the index of
    /// an earlier slot whose material is reused.
    pub indices: Vec<i32>,
    pub materials: Vec<Material>,
}

impl MaterialList {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW material list";
        let mut ch = Children::of(c, id::MATLIST, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let n = r.u32()? as usize;
        r.check_count(n, 4)?;
        let indices: Vec<i32> = (0..n).map(|_| r.i32()).collect::<Result<_>>()?;
        r.expect_end()?;
        let mut materials = Vec::new();
        for _ in indices.iter().filter(|&&i| i < 0) {
            materials.push(Material::from_chunk(ch.next(id::MATERIAL)?)?);
        }
        ch.finish()?;
        Ok(MaterialList {
            version: c.version,
            struct_version: s.version,
            indices,
            materials,
        })
    }

    pub fn to_chunk(&self) -> Chunk {
        let mut s = Vec::new();
        s.put_u32(self.indices.len() as u32);
        for i in &self.indices {
            s.put_i32(*i);
        }
        let mut children = vec![Chunk::new_data(id::STRUCT, self.struct_version, s)];
        children.extend(self.materials.iter().map(Material::to_chunk));
        Chunk::new_children(id::MATLIST, self.version, children)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub version: u32,
    pub struct_version: u32,
    /// First struct word (unused by RW 3.6 on load); kept raw.
    pub flags: u32,
    pub color: [u8; 4],
    /// Third struct word (unused); kept raw.
    pub unused: i32,
    /// Raw "is textured" word; a Texture chunk follows iff non-zero.
    pub textured: u32,
    /// Ambient, specular, diffuse; present for library versions > 0x30400.
    pub surface_props: Option<[f32; 3]>,
    pub texture: Option<Texture>,
    /// Extension (MatFX, R* reflection/specular, UV anim), raw.
    pub extension: Extension,
}

impl Material {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW material";
        let mut ch = Children::of(c, id::MATERIAL, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let flags = r.u32()?;
        let color = r.array()?;
        let unused = r.i32()?;
        let textured = r.u32()?;
        let surface_props = if LibraryStamp(s.version).version() > 0x30400 {
            Some(r.f32s()?)
        } else {
            None
        };
        r.expect_end()?;
        let texture = if textured != 0 {
            Some(Texture::from_chunk(ch.next(id::TEXTURE)?)?)
        } else {
            None
        };
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(Material {
            version: c.version,
            struct_version: s.version,
            flags,
            color,
            unused,
            textured,
            surface_props,
            texture,
            extension,
        })
    }

    pub fn to_chunk(&self) -> Chunk {
        let mut s = Vec::new();
        s.put_u32(self.flags);
        s.extend_from_slice(&self.color);
        s.put_i32(self.unused);
        s.put_u32(self.textured);
        if let Some(p) = &self.surface_props {
            s.put_f32s(p);
        }
        let mut children = vec![Chunk::new_data(id::STRUCT, self.struct_version, s)];
        children.extend(self.texture.iter().map(Texture::to_chunk));
        children.push(self.extension.to_chunk());
        Chunk::new_children(id::MATERIAL, self.version, children)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub version: u32,
    pub struct_version: u32,
    /// Filter mode in bits 0-7, U addressing in bits 8-11, V addressing in bits 12-15; the upper
    /// 16 bits are kept raw (meaning not confirmed for SA, see docs/re/formats.md).
    pub filter_addressing: u32,
    pub name: RwString,
    pub mask: RwString,
    pub extension: Extension,
}

impl Texture {
    pub fn filter(&self) -> u8 {
        self.filter_addressing as u8
    }
    pub fn address_u(&self) -> u8 {
        ((self.filter_addressing >> 8) & 0xF) as u8
    }
    pub fn address_v(&self) -> u8 {
        ((self.filter_addressing >> 12) & 0xF) as u8
    }

    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW texture";
        let mut ch = Children::of(c, id::TEXTURE, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let filter_addressing = r.u32()?;
        r.expect_end()?;
        let name = RwString::from_chunk(ch.next(id::STRING)?)?;
        let mask = RwString::from_chunk(ch.next(id::STRING)?)?;
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(Texture {
            version: c.version,
            struct_version: s.version,
            filter_addressing,
            name,
            mask,
            extension,
        })
    }

    pub fn to_chunk(&self) -> Chunk {
        Chunk::new_children(
            id::TEXTURE,
            self.version,
            vec![
                Chunk::new_data(
                    id::STRUCT,
                    self.struct_version,
                    self.filter_addressing.to_le_bytes().to_vec(),
                ),
                self.name.to_chunk(),
                self.mask.to_chunk(),
                self.extension.to_chunk(),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Atomic {
    pub version: u32,
    pub struct_version: u32,
    pub frame_index: i32,
    pub geometry_index: i32,
    /// `rpATOMIC*` flags (collision test 0x1, render 0x4).
    pub flags: u32,
    /// Fourth struct word (unused); kept raw.
    pub unused: u32,
    /// Embedded geometry (only when the clump has no geometry list entry for it; not in SA).
    pub geometry: Option<Geometry>,
    /// Extension (right-to-render, MatFX, pipeline set, env map...), raw.
    pub extension: Extension,
}

impl Atomic {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW atomic";
        let mut ch = Children::of(c, id::ATOMIC, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let (frame_index, geometry_index, flags, unused) = (r.i32()?, r.i32()?, r.u32()?, r.u32()?);
        r.expect_end()?;
        let geometry = if ch.peek_ty() == Some(id::GEOMETRY) {
            Some(Geometry::from_chunk(ch.next(id::GEOMETRY)?)?)
        } else {
            None
        };
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(Atomic {
            version: c.version,
            struct_version: s.version,
            frame_index,
            geometry_index,
            flags,
            unused,
            geometry,
            extension,
        })
    }

    pub fn to_chunk(&self) -> Result<Chunk> {
        let mut s = Vec::new();
        s.put_i32(self.frame_index);
        s.put_i32(self.geometry_index);
        s.put_u32(self.flags);
        s.put_u32(self.unused);
        let mut children = vec![Chunk::new_data(id::STRUCT, self.struct_version, s)];
        if let Some(g) = &self.geometry {
            children.push(g.to_chunk()?);
        }
        children.push(self.extension.to_chunk());
        Ok(Chunk::new_children(id::ATOMIC, self.version, children))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V: u32 = 0x1803FFFF;

    fn ext() -> Extension {
        Extension {
            version: V,
            plugins: vec![],
        }
    }

    fn sample_clump() -> Clump {
        let tex = Texture {
            version: V,
            struct_version: V,
            filter_addressing: 0x1106,
            name: RwString {
                version: V,
                raw: b"brick\0\0\0".to_vec(),
            },
            mask: RwString {
                version: V,
                raw: vec![0; 4],
            },
            extension: ext(),
        };
        let mat = Material {
            version: V,
            struct_version: V,
            flags: 0,
            color: [255, 128, 0, 255],
            unused: 0x1234,
            textured: 1,
            surface_props: Some([1.0, 1.0, 1.0]),
            texture: Some(tex),
            extension: Extension {
                version: V,
                plugins: vec![Chunk::new_data(id::MATERIALEFFECTSPLUGIN, V, vec![0; 8])],
            },
        };
        let data = GeometryData {
            format: geometry_flags::POSITIONS | geometry_flags::TEXTURED | geometry_flags::PRELIT | (1 << 16),
            num_triangles: 1,
            num_vertices: 3,
            surface_props: None,
            prelit: vec![[1, 2, 3, 4]; 3],
            tex_coords: vec![vec![[0.0, 1.0], [0.5, 0.5], [f32::from_bits(0x7FC0_0001), 0.0]]],
            triangles: vec![Triangle {
                v: [0, 1, 2],
                material: 0,
            }],
            morph_targets: vec![MorphTarget {
                bounding_sphere: [0.0, 0.0, 0.0, 2.0],
                has_vertices: 1,
                has_normals: 0,
                vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                normals: vec![],
            }],
        };
        let geom = Geometry {
            version: V,
            struct_version: V,
            data,
            material_list: MaterialList {
                version: V,
                struct_version: V,
                indices: vec![-1, 0],
                materials: vec![mat],
            },
            extension: ext(),
        };
        Clump {
            version: V,
            struct_version: V,
            has_light_camera_counts: true,
            frame_list: FrameList {
                version: V,
                struct_version: V,
                frames: vec![Frame {
                    rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    position: [1.0, 2.0, 3.0],
                    parent: -1,
                    matrix_flags: 0x20003,
                }],
                extensions: vec![ext()],
            },
            geometry_list: GeometryList {
                version: V,
                struct_version: V,
                geometries: vec![geom],
            },
            atomics: vec![Atomic {
                version: V,
                struct_version: V,
                frame_index: 0,
                geometry_index: 0,
                flags: 5,
                unused: 0,
                geometry: None,
                extension: ext(),
            }],
            lights: vec![ClumpObject {
                struct_version: V,
                frame_index: 0,
                chunk: Chunk::new_children(
                    id::LIGHT,
                    V,
                    vec![Chunk::new_data(id::STRUCT, V, vec![0; 24]), ext().to_chunk()],
                ),
            }],
            cameras: vec![],
            extension: ext(),
        }
    }

    #[test]
    fn clump_round_trip_through_bytes() {
        let clump = sample_clump();
        let bytes = clump.to_chunk().unwrap().to_bytes();
        let stream = super::super::RwStream::parse(&bytes).unwrap();
        assert!(stream.trailing.is_empty());
        let back = Clump::from_chunk(&stream.chunks[0]).unwrap();
        assert_eq!(back.to_chunk().unwrap().to_bytes(), bytes);
        let g = &back.geometry_list.geometries[0];
        assert_eq!(g.data.triangles[0].v, [0, 1, 2]);
        assert_eq!(g.data.tex_coords[0][2][0].to_bits(), 0x7FC0_0001);
        assert_eq!(
            g.material_list.materials[0]
                .texture
                .as_ref()
                .unwrap()
                .name
                .as_bytes(),
            b"brick"
        );
        assert_eq!(back.lights.len(), 1);
    }

    #[test]
    fn triangle_word_order() {
        let mut d = sample_clump().geometry_list.geometries[0].data.clone();
        d.triangles[0] = Triangle {
            v: [10, 11, 12],
            material: 3,
        };
        let b = d.to_bytes().unwrap();
        // header 16 + prelit 12 + uv 24 = 52
        assert_eq!(&b[52..60], &[11, 0, 10, 0, 3, 0, 12, 0]);
    }

    #[test]
    fn inconsistent_counts_rejected() {
        let mut d = sample_clump().geometry_list.geometries[0].data.clone();
        d.num_vertices = 4;
        assert!(d.to_bytes().is_err());
        let ok = sample_clump().geometry_list.geometries[0]
            .data
            .to_bytes()
            .unwrap();
        assert!(GeometryData::parse(&ok[..ok.len() - 1], V).is_err());
        let mut extra = ok.clone();
        extra.push(0);
        assert!(GeometryData::parse(&extra, V).is_err());
    }

    #[test]
    fn old_version_has_surface_props() {
        let mut d = sample_clump().geometry_list.geometries[0].data.clone();
        d.surface_props = Some([0.5, 0.25, 1.0]);
        let b = d.to_bytes().unwrap();
        // 0x30000-era stamp (unpacked 0x33000 < 0x34000)
        let back = GeometryData::parse(&b, 0x0C02FFFF).unwrap();
        assert_eq!(back.surface_props, d.surface_props);
        assert_eq!(back.to_bytes().unwrap(), b);
        // Without the surface properties (3.6 stamp) the same bytes do not parse.
        assert!(GeometryData::parse(&b, V).is_err());
    }
}
