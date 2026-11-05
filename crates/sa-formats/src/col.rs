//! Collision archives: COLL (v1), COL2, COL3, COL4. A file holds any number of models back to
//! back; reading stops (as in `CFileLoader::LoadCollisionFile`) when fewer than 8 bytes remain or
//! the next FourCC is not a COL tag. Whatever follows is kept as [`ColFile::trailing`].
//!
//! Layouts follow gta-reversed `ColHelpers.h` and `CFileLoader::LoadCollisionModel*`:
//!
//! * Every model starts with a 32-byte header: FourCC, `u32 size` (bytes after these 8),
//!   `char name[22]`, `u16 model id`.
//! * COLL: sequential sections (bounds, spheres, lines, boxes, vertices as `f32[3]`, faces).
//! * COL2+: a fixed header with counts, flags and offsets. Offsets are relative to byte 4 of the
//!   model (after the FourCC); 0 means absent. The game copies the body verbatim and only
//!   derives pointers from the offsets, so the body can contain bytes no offset points at
//!   (alignment padding, unreferenced vertices). This reader decodes every section the offsets
//!   describe and keeps every other byte in [`Gap`]s, so serialisation is byte-exact.
//! * Vertex arrays have no count in the file. The vertex section is taken to run up to the next
//!   section (or the end of the model); `region_len / 6` vertices are decoded and the remaining
//!   0..5 bytes are kept raw in `*_tail`.

use crate::bytes::{Put, Reader, c_str};
use crate::error::{Error, Result};

/// Size of the common model header (FourCC, size, name, model id).
pub const MODEL_HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColVersion {
    V1,
    V2,
    V3,
    V4,
}

impl ColVersion {
    pub fn from_fourcc(f: &[u8]) -> Option<Self> {
        match f {
            b"COLL" => Some(ColVersion::V1),
            b"COL2" => Some(ColVersion::V2),
            b"COL3" => Some(ColVersion::V3),
            b"COL4" => Some(ColVersion::V4),
            _ => None,
        }
    }

    pub fn fourcc(self) -> &'static [u8; 4] {
        match self {
            ColVersion::V1 => b"COLL",
            ColVersion::V2 => b"COL2",
            ColVersion::V3 => b"COL3",
            ColVersion::V4 => b"COL4",
        }
    }

    /// Size of the version-specific header that follows the 32-byte model header (COL2+).
    fn v2_header_size(self) -> usize {
        match self {
            ColVersion::V1 => 0,
            ColVersion::V2 => 0x4C,
            ColVersion::V3 => 88,
            ColVersion::V4 => 92,
        }
    }
}

/// `TSurface` / surface bytes of spheres, boxes and v1 faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Surface {
    pub material: u8,
    pub flag: u8,
    pub brightness: u8,
    pub light: u8,
}

impl Surface {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Surface {
            material: r.u8()?,
            flag: r.u8()?,
            brightness: r.u8()?,
            light: r.u8()?,
        })
    }
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[self.material, self.flag, self.brightness, self.light]);
    }
}

/// Box (28 bytes, all versions).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub surface: Surface,
}

impl ColBox {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ColBox {
            min: r.f32s()?,
            max: r.f32s()?,
            surface: Surface::read(r)?,
        })
    }
    fn write(&self, out: &mut Vec<u8>) {
        out.put_f32s(&self.min);
        out.put_f32s(&self.max);
        self.surface.write(out);
    }
}

/// Sphere (20 bytes). COLL stores radius first, COL2+ stores the centre first.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColSphere {
    pub center: [f32; 3],
    pub radius: f32,
    pub surface: Surface,
}

/// COLL bounds: sphere (radius, centre) then box (min, max).
/// COL2+ bounds: box (min, max) then sphere (centre, radius).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub center: [f32; 3],
    pub radius: f32,
}

/// COLL face (16 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceV1 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub surface: Surface,
}

/// COL2+ face (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceV2 {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub material: u8,
    pub light: u8,
}

impl FaceV2 {
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(FaceV2 {
            a: r.u16()?,
            b: r.u16()?,
            c: r.u16()?,
            material: r.u8()?,
            light: r.u8()?,
        })
    }
    fn write(&self, out: &mut Vec<u8>) {
        out.put_u16(self.a);
        out.put_u16(self.b);
        out.put_u16(self.c);
        out.push(self.material);
        out.push(self.light);
    }
}

/// Face group (28 bytes): bounding box of faces `first..=last`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FaceGroup {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub first: u16,
    pub last: u16,
}

/// Compressed vertex (`CompressedVector`: `int16[3]`, value = raw / 128).
pub type CompressedVertex = [i16; 3];

/// Decompresses a COL2+ vertex (`FixedVector<int16, 128.0f>`).
pub fn decompress_vertex(v: CompressedVertex) -> [f32; 3] {
    [v[0] as f32 / 128.0, v[1] as f32 / 128.0, v[2] as f32 / 128.0]
}

/// COLL model body.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColV1 {
    pub bounds: Bounds,
    pub spheres: Vec<ColSphere>,
    /// Lines, 24 bytes each (skipped by the game; none in the install), raw.
    pub lines: Vec<[u8; 24]>,
    pub boxes: Vec<ColBox>,
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<FaceV1>,
}

/// Bytes inside a COL2+ model that no section covers, at a model-relative byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub offset: u32,
    pub bytes: Vec<u8>,
}

/// COL3+ shadow mesh header fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShadowOffsets {
    pub off_vertices: u32,
    pub off_faces: u32,
}

/// COL2/COL3/COL4 model body.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColV2 {
    pub bounds: Bounds,
    /// Header byte after the line count (struct padding; 0 in the install), raw.
    pub pad: u8,
    /// Bit 1: uses disks (cleared by the game), bit 2: not empty, bit 8: has face groups,
    /// bit 16: has shadow mesh.
    pub flags: u32,
    pub off_spheres: u32,
    pub off_boxes: u32,
    pub off_lines: u32,
    pub off_vertices: u32,
    pub off_faces: u32,
    /// Triangle planes offset (0 in every file; the game ignores it), raw.
    pub off_planes: u32,
    /// COL3+ only.
    pub shadow: Option<ShadowOffsets>,
    /// COL4 only: the extra header word (unknown meaning, no COL4 in the install), raw.
    pub v4_unknown: Option<u32>,
    pub spheres: Vec<ColSphere>,
    pub boxes: Vec<ColBox>,
    pub vertices: Vec<CompressedVertex>,
    pub vertices_tail: Vec<u8>,
    /// Present when `flags & 8` and faces exist: stored right before the faces, followed by
    /// a `u32` count.
    pub face_groups: Option<Vec<FaceGroup>>,
    pub faces: Vec<FaceV2>,
    pub shadow_vertices: Vec<CompressedVertex>,
    pub shadow_vertices_tail: Vec<u8>,
    pub shadow_faces: Vec<FaceV2>,
    pub gaps: Vec<Gap>,
}

impl ColV2 {
    pub const FLAG_USES_DISKS: u32 = 1;
    pub const FLAG_NOT_EMPTY: u32 = 2;
    pub const FLAG_HAS_FACE_GROUPS: u32 = 8;
    pub const FLAG_HAS_SHADOW: u32 = 16;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColBody {
    V1(ColV1),
    /// COL2, COL3 or COL4 (see [`ColModel::version`]).
    V2(ColV2),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColModel {
    pub version: ColVersion,
    pub name_raw: [u8; 22],
    pub model_id: u16,
    pub body: ColBody,
}

/// A whole collision file (or IMG `.col` entry).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColFile {
    pub models: Vec<ColModel>,
    pub trailing: Vec<u8>,
}

/// Raw model slices (version, bytes including the header) and the trailing bytes.
pub type SplitModels<'a> = (Vec<(ColVersion, &'a [u8])>, &'a [u8]);

/// Splits `data` into raw model slices the way `CFileLoader::LoadCollisionFile` walks it.
/// Returns the model slices (including their headers) and the trailing bytes.
pub fn split_models(data: &[u8]) -> Result<SplitModels<'_>> {
    let mut models = Vec::new();
    let mut pos = 0;
    while data.len() - pos >= 8 {
        let Some(version) = ColVersion::from_fourcc(&data[pos..pos + 4]) else {
            break;
        };
        let size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let total = size + 8;
        if total < MODEL_HEADER_SIZE + version.v2_header_size() {
            return Err(Error::invalid(
                "COL model",
                format!("size {size} at {pos:#x} is smaller than the header"),
            ));
        }
        if total > data.len() - pos {
            return Err(Error::Truncated {
                context: "COL model",
                offset: pos,
                needed: total,
                available: data.len() - pos,
            });
        }
        models.push((version, &data[pos..pos + total]));
        pos += total;
    }
    Ok((models, &data[pos..]))
}

impl ColFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let (raw, trailing) = split_models(data)?;
        let models = raw
            .into_iter()
            .map(|(v, m)| ColModel::parse(v, m))
            .collect::<Result<_>>()?;
        Ok(ColFile {
            models,
            trailing: trailing.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for m in &self.models {
            out.extend_from_slice(&m.to_bytes()?);
        }
        out.extend_from_slice(&self.trailing);
        Ok(out)
    }
}

impl ColModel {
    pub fn name(&self) -> &[u8] {
        c_str(&self.name_raw)
    }

    /// Parses one model; `m` is exactly the model's bytes (header included).
    pub fn parse(version: ColVersion, m: &[u8]) -> Result<Self> {
        let mut r = Reader::new(m, "COL model header");
        r.bytes(8)?;
        let name_raw = r.array()?;
        let model_id = r.u16()?;
        let body = match version {
            ColVersion::V1 => ColBody::V1(parse_v1(&mut r)?),
            _ => ColBody::V2(parse_v2(version, m)?),
        };
        Ok(ColModel {
            version,
            name_raw,
            model_id,
            body,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.extend_from_slice(self.version.fourcc());
        out.put_u32(0); // patched below
        out.extend_from_slice(&self.name_raw);
        out.put_u16(self.model_id);
        match (&self.body, self.version) {
            (ColBody::V1(b), ColVersion::V1) => write_v1(b, &mut out),
            (ColBody::V2(b), v) if v != ColVersion::V1 => write_v2(b, v, &mut out)?,
            _ => {
                return Err(Error::invalid(
                    "COL model (write)",
                    "body does not match the version",
                ));
            }
        }
        let size = (out.len() - 8) as u32;
        out[4..8].copy_from_slice(&size.to_le_bytes());
        Ok(out)
    }
}

fn read_count(r: &mut Reader, item: usize) -> Result<usize> {
    let n = r.u32()? as usize;
    r.check_count(n, item)?;
    Ok(n)
}

fn parse_v1(r: &mut Reader) -> Result<ColV1> {
    r.set_context("COLL body");
    let radius = r.f32()?;
    let center = r.f32s()?;
    let bounds = Bounds {
        center,
        radius,
        min: r.f32s()?,
        max: r.f32s()?,
    };
    let n = read_count(r, 20)?;
    let mut spheres = Vec::with_capacity(n);
    for _ in 0..n {
        let radius = r.f32()?;
        spheres.push(ColSphere {
            radius,
            center: r.f32s()?,
            surface: Surface::read(r)?,
        });
    }
    let n = read_count(r, 24)?;
    let lines = (0..n).map(|_| r.array()).collect::<Result<_>>()?;
    let n = read_count(r, 28)?;
    let boxes = (0..n).map(|_| ColBox::read(r)).collect::<Result<_>>()?;
    let n = read_count(r, 12)?;
    let vertices = (0..n).map(|_| r.f32s()).collect::<Result<_>>()?;
    let n = read_count(r, 16)?;
    let faces = (0..n)
        .map(|_| {
            Ok(FaceV1 {
                a: r.u32()?,
                b: r.u32()?,
                c: r.u32()?,
                surface: Surface::read(r)?,
            })
        })
        .collect::<Result<_>>()?;
    r.expect_end()?;
    Ok(ColV1 {
        bounds,
        spheres,
        lines,
        boxes,
        vertices,
        faces,
    })
}

fn write_v1(b: &ColV1, out: &mut Vec<u8>) {
    out.put_f32(b.bounds.radius);
    out.put_f32s(&b.bounds.center);
    out.put_f32s(&b.bounds.min);
    out.put_f32s(&b.bounds.max);
    out.put_u32(b.spheres.len() as u32);
    for s in &b.spheres {
        out.put_f32(s.radius);
        out.put_f32s(&s.center);
        s.surface.write(out);
    }
    out.put_u32(b.lines.len() as u32);
    for l in &b.lines {
        out.extend_from_slice(l);
    }
    out.put_u32(b.boxes.len() as u32);
    for x in &b.boxes {
        x.write(out);
    }
    out.put_u32(b.vertices.len() as u32);
    for v in &b.vertices {
        out.put_f32s(v);
    }
    out.put_u32(b.faces.len() as u32);
    for f in &b.faces {
        out.put_u32(f.a);
        out.put_u32(f.b);
        out.put_u32(f.c);
        f.surface.write(out);
    }
}

/// Section kinds used while laying out a COL2+ body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sec {
    Spheres,
    Boxes,
    Vertices,
    FaceGroups,
    Faces,
    ShadowVertices,
    ShadowFaces,
}

/// Model-relative byte position of a section offset (offsets count from byte 4).
fn abs(off: u32) -> usize {
    off as usize + 4
}

fn parse_v2(version: ColVersion, m: &[u8]) -> Result<ColV2> {
    const CTX: &str = "COL2+ body";
    let header_end = MODEL_HEADER_SIZE + version.v2_header_size();
    let mut r = Reader::new(&m[MODEL_HEADER_SIZE..header_end], CTX);
    let mut b = ColV2 {
        bounds: Bounds {
            min: r.f32s()?,
            max: r.f32s()?,
            center: r.f32s()?,
            radius: r.f32()?,
        },
        ..Default::default()
    };
    let num_spheres = r.u16()? as usize;
    let num_boxes = r.u16()? as usize;
    let num_faces = r.u16()? as usize;
    let num_lines = r.u8()?;
    b.pad = r.u8()?;
    b.flags = r.u32()?;
    b.off_spheres = r.u32()?;
    b.off_boxes = r.u32()?;
    b.off_lines = r.u32()?;
    b.off_vertices = r.u32()?;
    b.off_faces = r.u32()?;
    b.off_planes = r.u32()?;
    let mut num_shadow_faces = 0;
    if version != ColVersion::V2 {
        num_shadow_faces = r.u32()? as usize;
        b.shadow = Some(ShadowOffsets {
            off_vertices: r.u32()?,
            off_faces: r.u32()?,
        });
    }
    if version == ColVersion::V4 {
        b.v4_unknown = Some(r.u32()?);
    }
    if num_lines != 0 {
        return Err(Error::unsupported(
            CTX,
            format!("{num_lines} lines/disks (none in the install)"),
        ));
    }
    if b.off_planes != 0 {
        return Err(Error::unsupported(
            CTX,
            "non-zero triangle plane offset (none in the install)",
        ));
    }

    // Fixed-size sections: (kind, start, len).
    let mut fixed: Vec<(Sec, usize, usize)> = Vec::new();
    let mut add = |kind, off: u32, len: usize| {
        if off != 0 {
            fixed.push((kind, abs(off), len));
        }
    };
    add(Sec::Spheres, b.off_spheres, num_spheres * 20);
    add(Sec::Boxes, b.off_boxes, num_boxes * 28);
    add(Sec::Faces, b.off_faces, num_faces * 8);
    if let Some(s) = b.shadow {
        add(Sec::ShadowFaces, s.off_faces, num_shadow_faces * 8);
    }
    let check_present = |n: usize, off: u32, what: &str| {
        if n != 0 && off == 0 {
            return Err(Error::invalid(CTX, format!("{n} {what} but offset 0")));
        }
        Ok(())
    };
    check_present(num_spheres, b.off_spheres, "spheres")?;
    check_present(num_boxes, b.off_boxes, "boxes")?;
    check_present(num_faces, b.off_faces, "faces")?;
    check_present(
        num_shadow_faces,
        b.shadow.map_or(0, |s| s.off_faces),
        "shadow faces",
    )?;

    let read_at = |start: usize, len: usize| -> Result<&[u8]> {
        if start < header_end || start + len > m.len() {
            return Err(Error::invalid(
                CTX,
                format!(
                    "section [{start:#x}, {:#x}) outside body [{header_end:#x}, {:#x})",
                    start + len,
                    m.len()
                ),
            ));
        }
        Ok(&m[start..start + len])
    };

    // Face groups: u32 count right before the faces, groups before that.
    if b.flags & ColV2::FLAG_HAS_FACE_GROUPS != 0 && b.off_faces != 0 {
        let faces = abs(b.off_faces);
        if faces < header_end + 4 {
            return Err(Error::invalid(CTX, "face group count before the body"));
        }
        let n = u32::from_le_bytes(read_at(faces - 4, 4)?.try_into().unwrap()) as usize;
        let len = n
            .checked_mul(28)
            .and_then(|l| l.checked_add(4))
            .filter(|&l| l <= faces - header_end);
        let Some(len) = len else {
            return Err(Error::invalid(
                CTX,
                format!("{n} face groups do not fit before the faces"),
            ));
        };
        let mut r = Reader::new(read_at(faces - len, len)?, CTX);
        let groups = (0..n)
            .map(|_| {
                Ok(FaceGroup {
                    min: r.f32s()?,
                    max: r.f32s()?,
                    first: r.u16()?,
                    last: r.u16()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        b.face_groups = Some(groups);
        fixed.push((Sec::FaceGroups, faces - len, len));
    }

    // Variable-length vertex regions run to the next non-empty section start or the model end.
    let region_end = |start: usize, fixed: &[(Sec, usize, usize)], other: Option<usize>| -> usize {
        fixed
            .iter()
            .filter(|&&(_, s, len)| len > 0 && s >= start)
            .map(|&(_, s, _)| s)
            .chain(other.filter(|&o| o > start))
            .min()
            .unwrap_or(m.len())
            .max(start)
    };
    let shadow_vert_start = b
        .shadow
        .filter(|s| s.off_vertices != 0)
        .map(|s| abs(s.off_vertices));
    let vert_start = (b.off_vertices != 0).then(|| abs(b.off_vertices));
    let mut regions = Vec::new();
    if let Some(start) = vert_start {
        regions.push((Sec::Vertices, start, region_end(start, &fixed, shadow_vert_start)));
    }
    if let Some(start) = shadow_vert_start {
        regions.push((Sec::ShadowVertices, start, region_end(start, &fixed, vert_start)));
    }

    // Decode sections.
    for &(kind, start, len) in &fixed {
        let mut r = Reader::new(read_at(start, len)?, CTX);
        match kind {
            Sec::Spheres => {
                b.spheres = (0..num_spheres)
                    .map(|_| {
                        Ok(ColSphere {
                            center: r.f32s()?,
                            radius: r.f32()?,
                            surface: Surface::read(&mut r)?,
                        })
                    })
                    .collect::<Result<_>>()?;
            }
            Sec::Boxes => {
                b.boxes = (0..num_boxes)
                    .map(|_| ColBox::read(&mut r))
                    .collect::<Result<_>>()?
            }
            Sec::Faces => {
                b.faces = (0..num_faces)
                    .map(|_| FaceV2::read(&mut r))
                    .collect::<Result<_>>()?
            }
            Sec::ShadowFaces => {
                b.shadow_faces = (0..num_shadow_faces)
                    .map(|_| FaceV2::read(&mut r))
                    .collect::<Result<_>>()?
            }
            _ => {}
        }
    }
    for &(kind, start, end) in &regions {
        let region = read_at(start, end - start)?;
        let n = region.len() / 6;
        let mut r = Reader::new(region, CTX);
        let verts = (0..n)
            .map(|_| Ok([r.i16()?, r.i16()?, r.i16()?]))
            .collect::<Result<Vec<_>>>()?;
        let tail = r.rest().to_vec();
        if kind == Sec::Vertices {
            (b.vertices, b.vertices_tail) = (verts, tail);
        } else {
            (b.shadow_vertices, b.shadow_vertices_tail) = (verts, tail);
        }
    }

    // Coverage: no overlaps; uncovered bytes become gaps.
    let mut spans: Vec<(usize, usize)> = fixed.iter().map(|&(_, s, l)| (s, s + l)).collect();
    spans.extend(regions.iter().map(|&(_, s, e)| (s, e)));
    spans.retain(|&(s, e)| e > s);
    spans.sort_unstable();
    let mut pos = header_end;
    for (s, e) in spans {
        if s < pos {
            return Err(Error::invalid(CTX, format!("sections overlap at {s:#x}")));
        }
        if s > pos {
            b.gaps.push(Gap {
                offset: pos as u32,
                bytes: m[pos..s].to_vec(),
            });
        }
        pos = e;
    }
    if pos < m.len() {
        b.gaps.push(Gap {
            offset: pos as u32,
            bytes: m[pos..].to_vec(),
        });
    }
    Ok(b)
}

fn write_v2(b: &ColV2, version: ColVersion, out: &mut Vec<u8>) -> Result<()> {
    const CTX: &str = "COL2+ body (write)";
    let to_u16 =
        |n: usize, what: &str| u16::try_from(n).map_err(|_| Error::invalid(CTX, format!("too many {what}")));
    if b.shadow.is_some() != (version != ColVersion::V2)
        || b.v4_unknown.is_some() != (version == ColVersion::V4)
    {
        return Err(Error::invalid(CTX, "header fields do not match the version"));
    }
    out.put_f32s(&b.bounds.min);
    out.put_f32s(&b.bounds.max);
    out.put_f32s(&b.bounds.center);
    out.put_f32(b.bounds.radius);
    out.put_u16(to_u16(b.spheres.len(), "spheres")?);
    out.put_u16(to_u16(b.boxes.len(), "boxes")?);
    out.put_u16(to_u16(b.faces.len(), "faces")?);
    out.put_u8(0);
    out.put_u8(b.pad);
    out.put_u32(b.flags);
    for o in [
        b.off_spheres,
        b.off_boxes,
        b.off_lines,
        b.off_vertices,
        b.off_faces,
        b.off_planes,
    ] {
        out.put_u32(o);
    }
    if let Some(s) = b.shadow {
        out.put_u32(b.shadow_faces.len() as u32);
        out.put_u32(s.off_vertices);
        out.put_u32(s.off_faces);
    }
    if let Some(u) = b.v4_unknown {
        out.put_u32(u);
    }

    // Lay out sections at their offsets.
    let mut pieces: Vec<(usize, Vec<u8>)> = Vec::new();
    let mut piece = |off: u32, data: Vec<u8>| -> Result<()> {
        if off == 0 {
            if !data.is_empty() {
                return Err(Error::invalid(CTX, "section has data but offset 0"));
            }
            return Ok(());
        }
        pieces.push((abs(off), data));
        Ok(())
    };
    let mut v = Vec::new();
    for s in &b.spheres {
        v.put_f32s(&s.center);
        v.put_f32(s.radius);
        s.surface.write(&mut v);
    }
    piece(b.off_spheres, v)?;
    let mut v = Vec::new();
    b.boxes.iter().for_each(|x| x.write(&mut v));
    piece(b.off_boxes, v)?;
    let verts = |list: &[CompressedVertex], tail: &[u8]| {
        let mut v = Vec::new();
        for x in list {
            x.iter().for_each(|c| v.put_i16(*c));
        }
        v.extend_from_slice(tail);
        v
    };
    piece(b.off_vertices, verts(&b.vertices, &b.vertices_tail))?;
    let mut v = Vec::new();
    b.faces.iter().for_each(|f| f.write(&mut v));
    let mut group_piece = None;
    if let Some(groups) = &b.face_groups {
        let mut g = Vec::new();
        for x in groups {
            g.put_f32s(&x.min);
            g.put_f32s(&x.max);
            g.put_u16(x.first);
            g.put_u16(x.last);
        }
        g.put_u32(groups.len() as u32);
        if b.off_faces == 0 || abs(b.off_faces) < g.len() {
            return Err(Error::invalid(CTX, "face groups without a face offset"));
        }
        group_piece = Some((abs(b.off_faces) - g.len(), g));
    }
    piece(b.off_faces, v)?;
    if let Some(s) = b.shadow {
        piece(s.off_vertices, verts(&b.shadow_vertices, &b.shadow_vertices_tail))?;
        let mut v = Vec::new();
        b.shadow_faces.iter().for_each(|f| f.write(&mut v));
        piece(s.off_faces, v)?;
    }
    pieces.extend(group_piece);
    for g in &b.gaps {
        pieces.push((g.offset as usize, g.bytes.clone()));
    }
    let header_end = out.len();
    let end = pieces
        .iter()
        .map(|(s, d)| s + d.len())
        .max()
        .unwrap_or(header_end)
        .max(header_end);
    out.resize(end, 0);
    let mut written: Vec<(usize, usize)> = Vec::new();
    for (s, d) in pieces {
        if d.is_empty() {
            continue;
        }
        if s < header_end {
            return Err(Error::invalid(
                CTX,
                format!("section at {s:#x} overlaps the header"),
            ));
        }
        out[s..s + d.len()].copy_from_slice(&d);
        written.push((s, s + d.len()));
    }
    written.sort_unstable();
    if written.windows(2).any(|w| w[1].0 < w[0].1) {
        return Err(Error::invalid(CTX, "sections overlap"));
    }
    Ok(())
}

/// Decodes the version text that `ValidateVersion` (0x5BA060) reads from `models/coll/peds.col`:
/// starting at byte 100, every 100th byte minus 23, up to a NUL (at most 128 characters).
/// Returns the text without the NUL, or `None` if the file ends first.
///
/// These stamp bytes overwrite model data in peds.col (see docs/re/formats.md).
pub fn peds_col_version_text(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for i in 0..128 {
        let c = data.get(100 + i * 100)?.wrapping_sub(23);
        if c == 0 {
            return Some(out);
        }
        out.push(c);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(m: u8) -> Surface {
        Surface {
            material: m,
            flag: 1,
            brightness: 2,
            light: 3,
        }
    }

    fn v1_model() -> ColModel {
        let mut name_raw = [0u8; 22];
        name_raw[..3].copy_from_slice(b"cop");
        name_raw[10] = 0x55;
        ColModel {
            version: ColVersion::V1,
            name_raw,
            model_id: 64,
            body: ColBody::V1(ColV1 {
                bounds: Bounds {
                    min: [-1.0; 3],
                    max: [1.0; 3],
                    center: [0.0; 3],
                    radius: 1.7,
                },
                spheres: vec![ColSphere {
                    center: [0.0, 0.0, 0.5],
                    radius: 0.3,
                    surface: surface(6),
                }],
                lines: vec![],
                boxes: vec![ColBox {
                    min: [0.0; 3],
                    max: [1.0; 3],
                    surface: surface(1),
                }],
                vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                faces: vec![FaceV1 {
                    a: 0,
                    b: 1,
                    c: 2,
                    surface: surface(4),
                }],
            }),
        }
    }

    /// Builds a COL3 model by hand: spheres, boxes, vertices (+2 pad), face groups, faces,
    /// shadow vertices, shadow faces.
    fn col3_bytes() -> Vec<u8> {
        let header_end = 32 + 88;
        let mut body = Vec::new();
        // spheres @ header_end
        let off_spheres = header_end - 4;
        body.put_f32s(&[1.0, 2.0, 3.0, 0.5]);
        body.extend_from_slice(&[9, 0, 0, 0]);
        // boxes
        let off_boxes = header_end + body.len() - 4;
        body.put_f32s(&[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        body.extend_from_slice(&[1, 0, 0, 0]);
        // 3 vertices + 2 bytes alignment padding
        let off_vertices = header_end + body.len() - 4;
        for v in [[0i16, 0, 0], [128, 0, 0], [0, 128, 0]] {
            v.iter().for_each(|c| body.put_i16(*c));
        }
        body.extend_from_slice(&[0xAB, 0xCD]);
        // one face group + count
        body.put_f32s(&[0.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
        body.put_u16(0);
        body.put_u16(0);
        body.put_u32(1);
        let off_faces = header_end + body.len() - 4;
        body.extend_from_slice(&[0, 0, 1, 0, 2, 0, 5, 7]);
        let off_shadow_vertices = header_end + body.len() - 4;
        for v in [[0i16, 0, 1], [1, 0, 1], [0, 1, 1]] {
            v.iter().for_each(|c| body.put_i16(*c));
        }
        let off_shadow_faces = header_end + body.len() - 4;
        body.extend_from_slice(&[0, 0, 1, 0, 2, 0, 5, 7]);
        body.extend_from_slice(&[0xEE; 3]); // unexplained trailing bytes -> gap

        let mut m = Vec::new();
        m.extend_from_slice(b"COL3");
        m.put_u32((header_end + body.len() - 8) as u32);
        let mut name = [0u8; 22];
        name[..4].copy_from_slice(b"test");
        m.extend_from_slice(&name);
        m.put_u16(1234);
        m.put_f32s(&[-1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.8]);
        m.put_u16(1);
        m.put_u16(1);
        m.put_u16(1);
        m.put_u8(0);
        m.put_u8(0);
        m.put_u32(2 | 8 | 16);
        for o in [
            off_spheres,
            off_boxes,
            0,
            off_vertices,
            off_faces,
            0,
            1,
            off_shadow_vertices,
            off_shadow_faces,
        ] {
            m.put_u32(o as u32);
        }
        assert_eq!(m.len(), header_end);
        m.extend_from_slice(&body);
        m
    }

    #[test]
    fn v1_round_trip() {
        let model = v1_model();
        let bytes = model.to_bytes().unwrap();
        let file = ColFile::parse(&bytes).unwrap();
        assert_eq!(file.models, vec![model]);
        assert_eq!(file.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn col3_sections_and_gaps() {
        let mut bytes = col3_bytes();
        let model_len = bytes.len();
        bytes.extend_from_slice(&[0; 100]); // sector padding
        let file = ColFile::parse(&bytes).unwrap();
        assert_eq!(file.trailing.len(), 100);
        let ColBody::V2(b) = &file.models[0].body else {
            panic!()
        };
        assert_eq!(file.models[0].name(), b"test");
        assert_eq!(b.spheres[0].radius, 0.5);
        assert_eq!(b.vertices.len(), 3);
        assert_eq!(b.vertices_tail, vec![0xAB, 0xCD]);
        assert_eq!(decompress_vertex(b.vertices[1]), [1.0, 0.0, 0.0]);
        assert_eq!(b.face_groups.as_ref().unwrap().len(), 1);
        assert_eq!(
            b.faces[0],
            FaceV2 {
                a: 0,
                b: 1,
                c: 2,
                material: 5,
                light: 7
            }
        );
        assert_eq!(b.shadow_vertices.len(), 3);
        assert_eq!(b.shadow_faces.len(), 1);
        assert_eq!(
            b.gaps,
            vec![Gap {
                offset: (model_len - 3) as u32,
                bytes: vec![0xEE; 3]
            }]
        );
        assert_eq!(file.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn multiple_models_and_bad_fourcc_stops() {
        let mut bytes = v1_model().to_bytes().unwrap();
        bytes.extend_from_slice(&col3_bytes());
        bytes.extend_from_slice(b"OLL\x01garbage");
        let file = ColFile::parse(&bytes).unwrap();
        assert_eq!(file.models.len(), 2);
        assert_eq!(file.trailing, b"OLL\x01garbage");
        assert_eq!(file.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn version_text() {
        let mut data = vec![0u8; 1000];
        for (i, c) in b"gta3\0".iter().enumerate() {
            data[100 + i * 100] = c.wrapping_add(23);
        }
        assert_eq!(peds_col_version_text(&data).unwrap(), b"gta3");
        assert_eq!(peds_col_version_text(&data[..300]), None);
    }

    #[test]
    fn errors() {
        // Declared size larger than the data.
        let mut b = v1_model().to_bytes().unwrap();
        b.truncate(b.len() - 1);
        assert!(matches!(ColFile::parse(&b), Err(Error::Truncated { .. })));
        // COLL count that overruns the model (like peds.col "cop": 133 boxes, no box data).
        let mut b = v1_model().to_bytes().unwrap();
        let box_count_at = 32 + 40 + 4 + 20 + 4;
        b[box_count_at] = 133;
        assert!(ColFile::parse(&b).is_err());
        // Overlapping sections.
        let mut b = col3_bytes();
        let off_boxes_at = 32 + 52 + 4;
        let off_spheres = u32::from_le_bytes(b[32 + 52..32 + 56].try_into().unwrap());
        b[off_boxes_at..off_boxes_at + 4].copy_from_slice(&(off_spheres + 4).to_le_bytes());
        assert!(ColFile::parse(&b).is_err());
    }
}
