//! `RpGeometry`, `RpMaterial`, `RpMeshHeader`/`RpMesh` (bin mesh plugin) and the extra vertex
//! colour plugin.

use sa_formats::rw::dff;

use crate::{Result, ids, lower_name, u32_at};

pub use dff::geometry_flags as flags;

/// The texture reference of a material (`RwTexture` as the DFF names it; the raster comes
/// from a texture dictionary at render setup).
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialTexture {
    /// Lower-case name.
    pub name: String,
    /// Lower-case mask name (empty when none).
    pub mask: String,
    /// `RwTexture::filterAddressing`: filter in bits 0-7, U addressing 8-11, V addressing 12-15.
    pub filter_addressing: u32,
}

impl MaterialTexture {
    /// `rwFILTER*` (1 nearest, 2 linear, 3 mip nearest, 4 mip linear, 5 linear mip nearest,
    /// 6 linear mip linear).
    pub fn filter(&self) -> u8 {
        (self.filter_addressing & 0xFF) as u8
    }
    /// `rwTEXTUREADDRESS*` (1 wrap, 2 mirror, 3 clamp, 4 border).
    pub fn address_u(&self) -> u8 {
        ((self.filter_addressing >> 8) & 0xF) as u8
    }
    pub fn address_v(&self) -> u8 {
        ((self.filter_addressing >> 12) & 0xF) as u8
    }
}

/// `RpMaterial` (0x1c bytes in the original: texture, color, pipeline, surfaceProps, refCount).
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// `RwRGBA color`.
    pub color: [u8; 4],
    /// `RwSurfaceProperties` (ambient, specular, diffuse); RW's default (1, 1, 1) when absent.
    pub surface_props: [f32; 3],
    pub texture: Option<MaterialTexture>,
    /// Whether the extension carries a MatFX (0x120) or R* env/specular plugin (not rendered yet).
    pub has_effects: bool,
}

/// One `RpMesh`: the indices of one material as the bin mesh plugin stores them.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Index into [`Geometry::materials`].
    pub material: usize,
    /// Triangle list indices (tristrips are converted, degenerate triangles dropped).
    pub indices: Vec<u32>,
}

/// `RpGeometry` with its first morph target (`RpMorphTarget`) inlined.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// `rpGEOMETRY*` flags (low byte of the format word).
    pub flags: u32,
    pub num_vertices: usize,
    /// Bounding sphere of morph target 0: centre, radius.
    pub bounding_sphere: [f32; 4],
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    /// Texture coordinate set 0 (`texCoords[0]`).
    pub uv0: Option<Vec<[f32; 2]>>,
    /// `preLitLum`: the (day) prelight colours.
    pub prelit: Option<Vec<[u8; 4]>>,
    /// Night prelight colours of the extra vertex colour plugin (0x253F2F9).
    pub night_colors: Option<Vec<[u8; 4]>>,
    pub materials: Vec<Material>,
    /// From the bin mesh plugin when present, else built from the triangle list.
    pub meshes: Vec<Mesh>,
    /// Whether the bin mesh plugin stored tristrips (`rpMESHHEADERTRISTRIP`).
    pub tristrip: bool,
}

impl Geometry {
    pub fn modulate_material_color(&self) -> bool {
        self.flags & flags::MODULATE_MATERIAL_COLOR != 0
    }

    pub fn from_dff(g: &dff::Geometry) -> Result<Geometry> {
        let d = &g.data;
        if d.is_native() {
            return Err(crate::Error::Invalid("native (platform) geometry is not supported".into()));
        }
        let n = d.num_vertices.max(0) as usize;
        let mt = d.morph_targets.first();
        let positions = mt.map(|m| m.vertices.clone()).unwrap_or_default();
        let normals = mt.filter(|m| !m.normals.is_empty()).map(|m| m.normals.clone());
        let bounding_sphere = mt.map(|m| m.bounding_sphere).unwrap_or([0.0; 4]);
        let uv0 = d.tex_coords.first().cloned();
        let prelit = (!d.prelit.is_empty()).then(|| d.prelit.clone());

        let materials = expand_material_list(&g.material_list);
        let night_colors = g
            .extension
            .find(ids::EXTRAVERTCOLOURPLUGIN)
            .and_then(|c| c.data())
            .and_then(|b| parse_night_colors(b, n));

        let mut tristrip = false;
        let meshes = match g.extension.find(ids::BINMESHPLUGIN).and_then(|c| c.data()) {
            Some(b) => match parse_bin_mesh(b) {
                Some((strip, meshes)) => {
                    tristrip = strip;
                    meshes
                }
                None => meshes_from_triangles(d, materials.len()),
            },
            None => meshes_from_triangles(d, materials.len()),
        };
        let meshes = meshes
            .into_iter()
            .filter(|m| m.material < materials.len() && m.indices.iter().all(|&i| (i as usize) < n))
            .collect();

        Ok(Geometry {
            flags: d.flags(),
            num_vertices: n,
            bounding_sphere,
            positions,
            normals,
            uv0,
            prelit,
            night_colors,
            materials,
            meshes,
            tristrip,
        })
    }
}

/// `RpMaterialList` slots: `-1` = next new material, otherwise a reference to an earlier slot.
fn expand_material_list(l: &dff::MaterialList) -> Vec<Material> {
    let mut out: Vec<Material> = Vec::with_capacity(l.indices.len());
    let mut next = l.materials.iter();
    for &ix in &l.indices {
        let m = if ix < 0 {
            next.next().map(material_from_dff)
        } else {
            out.get(ix as usize).cloned()
        };
        out.push(m.unwrap_or(Material {
            color: [255; 4],
            surface_props: [1.0; 3],
            texture: None,
            has_effects: false,
        }));
    }
    out
}

fn material_from_dff(m: &dff::Material) -> Material {
    let has_effects = [ids::MATERIALEFFECTSPLUGIN, ids::ENVMAPPLUGIN, ids::SPECMAPPLUGIN]
        .iter()
        .any(|&t| m.extension.find(t).is_some());
    Material {
        color: m.color,
        surface_props: m.surface_props.unwrap_or([1.0; 3]),
        texture: m.texture.as_ref().map(|t| MaterialTexture {
            name: lower_name(t.name.as_bytes()),
            mask: lower_name(t.mask.as_bytes()),
            filter_addressing: t.filter_addressing,
        }),
        has_effects,
    }
}

/// Extra vertex colour plugin stream data: `u32 hasColours`, then `RwRGBA[numVertices]`.
fn parse_night_colors(b: &[u8], n: usize) -> Option<Vec<[u8; 4]>> {
    if u32_at(b, 0)? == 0 {
        return None;
    }
    let body = b.get(4..4 + n * 4)?;
    Some(body.as_chunks::<4>().0.to_vec())
}

/// Bin mesh plugin stream data: `u32 flags` (1 = tristrip), `u32 numMeshes`, `u32 totalIndices`,
/// then per mesh `u32 numIndices`, `u32 materialIndex`, `u32 indices[numIndices]`.
fn parse_bin_mesh(b: &[u8]) -> Option<(bool, Vec<Mesh>)> {
    let strip = u32_at(b, 0)? & 1 != 0;
    let count = u32_at(b, 4)? as usize;
    let mut off = 12;
    let mut meshes = Vec::with_capacity(count);
    for _ in 0..count {
        let ni = u32_at(b, off)? as usize;
        let mat = u32_at(b, off + 4)? as usize;
        off += 8;
        let raw: Vec<u32> = (0..ni).map(|k| u32_at(b, off + 4 * k)).collect::<Option<_>>()?;
        off += 4 * ni;
        let indices = if strip { strip_to_list(&raw) } else { raw };
        meshes.push(Mesh { material: mat, indices });
    }
    Some((strip, meshes))
}

/// Tristrip → triangle list with alternating winding; degenerate triangles are dropped.
pub fn strip_to_list(s: &[u32]) -> Vec<u32> {
    let mut out = Vec::with_capacity(s.len().saturating_sub(2) * 3);
    for i in 2..s.len() {
        let (a, b, c) = if i % 2 == 0 { (s[i - 2], s[i - 1], s[i]) } else { (s[i - 1], s[i - 2], s[i]) };
        if a != b && b != c && a != c {
            out.extend_from_slice(&[a, b, c]);
        }
    }
    out
}

fn meshes_from_triangles(d: &dff::GeometryData, num_materials: usize) -> Vec<Mesh> {
    let mut meshes: Vec<Mesh> = (0..num_materials).map(|m| Mesh { material: m, indices: Vec::new() }).collect();
    for t in &d.triangles {
        if let Some(m) = meshes.get_mut(t.material as usize) {
            m.indices.extend(t.v.iter().map(|&v| v as u32));
        }
    }
    meshes.retain(|m| !m.indices.is_empty());
    meshes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_conversion_alternates_and_drops_degenerates() {
        assert_eq!(strip_to_list(&[0, 1, 2, 3]), vec![0, 1, 2, 2, 1, 3]);
        assert_eq!(strip_to_list(&[0, 1, 1, 2]), Vec::<u32>::new());
    }

    #[test]
    fn bin_mesh_parses() {
        let mut b = Vec::new();
        for w in [0u32, 1, 3, 3, 0, 0, 1, 2] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        let (strip, m) = parse_bin_mesh(&b).unwrap();
        assert!(!strip);
        assert_eq!(m, vec![Mesh { material: 0, indices: vec![0, 1, 2] }]);
    }

    #[test]
    fn night_colours_parse() {
        let mut b = 1u32.to_le_bytes().to_vec();
        b.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(parse_night_colors(&b, 2), Some(vec![[1, 2, 3, 4], [5, 6, 7, 8]]));
        assert_eq!(parse_night_colors(&0u32.to_le_bytes(), 2), None);
    }
}
