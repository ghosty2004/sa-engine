//! `RwTexDictionary`, `RwTexture`, `RwRaster` from D3D8/D3D9 native textures.

use std::sync::Arc;

use sa_formats::rw::txd::{self, Compression, TextureNative, raster_format as rf};
use sa_formats::rw::{RwStream, id};

use crate::{Error, Result, dxt, lower_name};

/// Pixel data of a raster, one entry per mip level (largest first).
#[derive(Debug, Clone, PartialEq)]
pub enum RasterData {
    /// DXT1 blocks (BC1).
    Bc1(Vec<Vec<u8>>),
    /// DXT3 blocks (BC2).
    Bc2(Vec<Vec<u8>>),
    /// DXT5 blocks (BC3).
    Bc3(Vec<Vec<u8>>),
    /// RGBA8, converted from the D3D formats (`A8R8G8B8`, `X8R8G8B8`, `A1R5G5B5`, `R5G6B5`,
    /// `A4R4G4B4`, `L8`, `PAL8`).
    Rgba8(Vec<Vec<u8>>),
}

impl RasterData {
    pub fn levels(&self) -> &[Vec<u8>] {
        match self {
            RasterData::Bc1(l) | RasterData::Bc2(l) | RasterData::Bc3(l) | RasterData::Rgba8(l) => l,
        }
    }
}

/// `RwRaster` (texture rasters only).
#[derive(Debug, Clone, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    /// `rwRASTERFORMAT*` as stored (pixel format, mip flags).
    pub format: u32,
    /// The D3D "alpha" flag of the native texture (`_rwD3D9RasterExt::alpha`).
    pub has_alpha: bool,
    pub data: RasterData,
}

impl Raster {
    pub fn num_levels(&self) -> usize {
        self.data.levels().len()
    }

    /// Mip level `i` as RGBA8 (decoding DXT in software).
    pub fn level_rgba8(&self, i: usize) -> Vec<u8> {
        let (w, h) = level_size(self.width, self.height, i);
        match &self.data {
            RasterData::Rgba8(l) => l[i].clone(),
            RasterData::Bc1(l) => dxt::decode(dxt::Kind::Bc1, &l[i], w, h),
            RasterData::Bc2(l) => dxt::decode(dxt::Kind::Bc2, &l[i], w, h),
            RasterData::Bc3(l) => dxt::decode(dxt::Kind::Bc3, &l[i], w, h),
        }
    }

    pub fn from_native(t: &TextureNative) -> Result<Raster> {
        let (w, h) = (t.width as u32, t.height as u32);
        if w == 0 || h == 0 {
            return Err(Error::Invalid("zero-sized raster".into()));
        }
        let comp = t.compression();
        let pixel = t.raster_format & rf::PIXEL_FORMAT_MASK;
        let mut levels = Vec::new();
        for (i, l) in t.levels.iter().enumerate() {
            let (lw, lh) = level_size(w, h, i);
            let need = match comp {
                Compression::Dxt1 => blocks(lw) * blocks(lh) * 8,
                Compression::None => {
                    if t.raster_format & rf::PAL8 != 0 || pixel == rf::LUM8 {
                        (lw * lh) as usize
                    } else if matches!(pixel, rf::C1555 | rf::C565 | rf::C4444 | rf::C555) {
                        (lw * lh * 2) as usize
                    } else {
                        (lw * lh * 4) as usize
                    }
                }
                _ => blocks(lw) * blocks(lh) * 16,
            };
            if l.len() < need {
                break;
            }
            levels.push(&l[..need]);
        }
        if levels.is_empty() {
            return Err(Error::Invalid("raster has no complete mip level".into()));
        }
        let data = match comp {
            Compression::Dxt1 => RasterData::Bc1(levels.iter().map(|l| l.to_vec()).collect()),
            Compression::Dxt2 | Compression::Dxt3 => RasterData::Bc2(levels.iter().map(|l| l.to_vec()).collect()),
            Compression::Dxt4 | Compression::Dxt5 => RasterData::Bc3(levels.iter().map(|l| l.to_vec()).collect()),
            Compression::None => {
                let pal = t.palette.as_deref();
                RasterData::Rgba8(
                    levels
                        .iter()
                        .map(|l| convert_uncompressed(l, t.raster_format, pal))
                        .collect::<Result<_>>()?,
                )
            }
        };
        Ok(Raster { width: w, height: h, format: t.raster_format, has_alpha: t.has_alpha(), data })
    }
}

/// Size of mip level `i` (each dimension halves, minimum 1).
pub fn level_size(w: u32, h: u32, i: usize) -> (u32, u32) {
    ((w >> i).max(1), (h >> i).max(1))
}

fn blocks(n: u32) -> usize {
    n.div_ceil(4) as usize
}

fn convert_uncompressed(l: &[u8], format: u32, pal: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(l.len() * 4);
    if format & rf::PAL8 != 0 {
        let pal = pal.ok_or_else(|| Error::Invalid("PAL8 raster without palette".into()))?;
        for &i in l {
            let p = &pal[i as usize * 4..i as usize * 4 + 4];
            out.extend_from_slice(p);
        }
        return Ok(out);
    }
    let ext5 = |v: u16| ((v as u32 * 255 + 15) / 31) as u8;
    let ext6 = |v: u16| ((v as u32 * 255 + 31) / 63) as u8;
    match format & rf::PIXEL_FORMAT_MASK {
        rf::C8888 => l.as_chunks::<4>().0.iter().for_each(|p| out.extend_from_slice(&[p[2], p[1], p[0], p[3]])),
        rf::C888 => l.as_chunks::<4>().0.iter().for_each(|p| out.extend_from_slice(&[p[2], p[1], p[0], 255])),
        rf::LUM8 => l.iter().for_each(|&v| out.extend_from_slice(&[v, v, v, 255])),
        rf::C1555 => l.as_chunks::<2>().0.iter().for_each(|p| {
            let v = u16::from_le_bytes(*p);
            let a = if v & 0x8000 != 0 { 255 } else { 0 };
            out.extend_from_slice(&[ext5((v >> 10) & 31), ext5((v >> 5) & 31), ext5(v & 31), a]);
        }),
        rf::C555 => l.as_chunks::<2>().0.iter().for_each(|p| {
            let v = u16::from_le_bytes(*p);
            out.extend_from_slice(&[ext5((v >> 10) & 31), ext5((v >> 5) & 31), ext5(v & 31), 255]);
        }),
        rf::C565 => l.as_chunks::<2>().0.iter().for_each(|p| {
            let v = u16::from_le_bytes(*p);
            out.extend_from_slice(&[ext5(v >> 11), ext6((v >> 5) & 63), ext5(v & 31), 255]);
        }),
        rf::C4444 => l.as_chunks::<2>().0.iter().for_each(|p| {
            let v = u16::from_le_bytes(*p);
            let x = |s: u16| (((v >> s) & 15) * 17) as u8;
            out.extend_from_slice(&[x(8), x(4), x(0), x(12)]);
        }),
        f => return Err(Error::Invalid(format!("unsupported raster pixel format {f:#x}"))),
    }
    Ok(out)
}

/// `RwTexture`.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// Lower-case name.
    pub name: String,
    pub mask: String,
    /// `filterAddressing` of the native texture.
    pub filter_addressing: u32,
    pub raster: Arc<Raster>,
}

/// `RwTexDictionary`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TexDictionary {
    pub textures: Vec<Texture>,
}

impl TexDictionary {
    /// `RwTexDictionaryFindNamedTexture` (case-insensitive; `name` must be lower-case).
    pub fn find(&self, name: &str) -> Option<&Texture> {
        self.textures.iter().find(|t| t.name == name)
    }

    pub fn from_txd(d: &txd::TexDictionary) -> TexDictionary {
        // A texture whose raster cannot be decoded is skipped (as a missing texture).
        let textures = d
            .textures
            .iter()
            .filter_map(|t| {
                Some(Texture {
                    name: lower_name(t.name()),
                    mask: lower_name(t.mask()),
                    filter_addressing: t.filter_addressing,
                    raster: Arc::new(Raster::from_native(t).ok()?),
                })
            })
            .collect();
        TexDictionary { textures }
    }
}

/// Reads a TXD stream (`RwTexDictionaryStreamRead`).
pub fn read_tex_dictionary(data: &[u8]) -> Result<TexDictionary> {
    let stream = RwStream::parse(data)?;
    let c = stream.chunks.iter().find(|c| c.ty == id::TEXDICTIONARY).ok_or(Error::Missing("texture dictionary"))?;
    Ok(TexDictionary::from_txd(&txd::TexDictionary::from_chunk(c)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgra_and_565() {
        let v = convert_uncompressed(&[1, 2, 3, 4], rf::C8888, None).unwrap();
        assert_eq!(v, vec![3, 2, 1, 4]);
        let v = convert_uncompressed(&0xF800u16.to_le_bytes(), rf::C565, None).unwrap();
        assert_eq!(v, vec![255, 0, 0, 255]);
        let v = convert_uncompressed(&0x8000u16.to_le_bytes(), rf::C1555, None).unwrap();
        assert_eq!(v, vec![0, 0, 0, 255]);
    }

    #[test]
    fn level_sizes_clamp_to_one() {
        assert_eq!(level_size(8, 2, 2), (2, 1));
        assert_eq!(level_size(8, 2, 5), (1, 1));
    }
}
