//! Typed decoding of TXD files (texture dictionaries with D3D8/D3D9 native textures).
//!
//! Native texture struct (librw `d3d/d3d.cpp` `readNativeTexture`, gtamods "Texture Native Struct";
//! verified by the byte-exact typed round trip over every TXD in the 1.0 US install):
//!
//! | field | D3D9 (platform 9) | D3D8 (platform 8) |
//! |---|---|---|
//! | u32 | platform id | platform id |
//! | u32 | filter (bits 0-7), U addressing (8-11), V addressing (12-15) | same |
//! | char[32], char[32] | name, mask name | same |
//! | u32 | raster format (`rwRASTERFORMAT*`) | same |
//! | u32 | `D3DFORMAT` (FourCC for DXTn) | "has alpha" |
//! | u16, u16 | width, height | same |
//! | u8, u8, u8 | depth, mip level count, raster type | same |
//! | u8 | flags: bit0 alpha, bit1 cube, bit2 auto mipmaps, bit3 compressed | DXT compression (0 = none, 1-5 = DXTn) |
//! | palette | 256 RGBA entries if `PAL8` | same |
//! | per level | u32 size + data | same |
//!
//! `PAL4` palettes and cube textures do not occur in the install and are rejected as unsupported.

use super::chunk::Children;
use super::{Chunk, Extension, id};
use crate::bytes::{Put, Reader, c_str};
use crate::error::{Error, Result};

/// `rwRASTERFORMAT*` values (gta-reversed `rwcore.h`).
pub mod raster_format {
    pub const DEFAULT: u32 = 0x0000;
    pub const C1555: u32 = 0x0100;
    pub const C565: u32 = 0x0200;
    pub const C4444: u32 = 0x0300;
    pub const LUM8: u32 = 0x0400;
    pub const C8888: u32 = 0x0500;
    pub const C888: u32 = 0x0600;
    pub const C16: u32 = 0x0700;
    pub const C24: u32 = 0x0800;
    pub const C32: u32 = 0x0900;
    pub const C555: u32 = 0x0A00;
    pub const AUTOMIPMAP: u32 = 0x1000;
    pub const PAL8: u32 = 0x2000;
    pub const PAL4: u32 = 0x4000;
    pub const MIPMAP: u32 = 0x8000;
    pub const PIXEL_FORMAT_MASK: u32 = 0x0F00;
}

pub const PLATFORM_D3D8: u32 = 8;
pub const PLATFORM_D3D9: u32 = 9;

/// FourCC `D3DFORMAT` values for DXT compression.
pub const D3DFMT_DXT1: u32 = u32::from_le_bytes(*b"DXT1");
pub const D3DFMT_DXT3: u32 = u32::from_le_bytes(*b"DXT3");
pub const D3DFMT_DXT5: u32 = u32::from_le_bytes(*b"DXT5");

/// DXT compression of a texture as derived from its platform fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Dxt1,
    Dxt2,
    Dxt3,
    Dxt4,
    Dxt5,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureNative {
    pub version: u32,
    pub struct_version: u32,
    pub platform: u32,
    pub filter_addressing: u32,
    pub name_raw: [u8; 32],
    pub mask_raw: [u8; 32],
    pub raster_format: u32,
    /// D3D9: `D3DFORMAT`; D3D8: "has alpha" word.
    pub format_word: u32,
    pub width: u16,
    pub height: u16,
    pub depth: u8,
    pub raster_type: u8,
    /// D3D9: flags byte; D3D8: compression byte.
    pub flags_byte: u8,
    /// 256 RGBA entries (1024 bytes) when `raster_format & PAL8`.
    pub palette: Option<Vec<u8>>,
    /// Mip levels, largest first.
    pub levels: Vec<Vec<u8>>,
    pub extension: Extension,
}

impl TextureNative {
    pub fn name(&self) -> &[u8] {
        c_str(&self.name_raw)
    }

    pub fn mask(&self) -> &[u8] {
        c_str(&self.mask_raw)
    }

    pub fn has_alpha(&self) -> bool {
        match self.platform {
            PLATFORM_D3D9 => self.flags_byte & 1 != 0,
            _ => self.format_word != 0,
        }
    }

    pub fn compression(&self) -> Compression {
        let from_d3d8 = |c: u8| match c {
            1 => Compression::Dxt1,
            2 => Compression::Dxt2,
            3 => Compression::Dxt3,
            4 => Compression::Dxt4,
            5 => Compression::Dxt5,
            _ => Compression::None,
        };
        match self.platform {
            PLATFORM_D3D9 if self.flags_byte & 8 != 0 => match self.format_word {
                D3DFMT_DXT1 => Compression::Dxt1,
                D3DFMT_DXT3 => Compression::Dxt3,
                D3DFMT_DXT5 => Compression::Dxt5,
                f if f == u32::from_le_bytes(*b"DXT2") => Compression::Dxt2,
                f if f == u32::from_le_bytes(*b"DXT4") => Compression::Dxt4,
                _ => Compression::None,
            },
            PLATFORM_D3D9 => Compression::None,
            _ => from_d3d8(self.flags_byte),
        }
    }

    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW texture native";
        let mut ch = Children::of(c, id::TEXTURENATIVE, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let platform = r.u32()?;
        if platform != PLATFORM_D3D8 && platform != PLATFORM_D3D9 {
            return Err(Error::unsupported(CTX, format!("platform {platform}")));
        }
        let filter_addressing = r.u32()?;
        let name_raw = r.array()?;
        let mask_raw = r.array()?;
        let raster_format = r.u32()?;
        let format_word = r.u32()?;
        let width = r.u16()?;
        let height = r.u16()?;
        let depth = r.u8()?;
        let num_levels = r.u8()?;
        let raster_type = r.u8()?;
        let flags_byte = r.u8()?;
        if raster_format & raster_format::PAL4 != 0 {
            return Err(Error::unsupported(
                CTX,
                "PAL4 palette (palette size not verified)",
            ));
        }
        if platform == PLATFORM_D3D9 && flags_byte & 2 != 0 {
            return Err(Error::unsupported(CTX, "cube texture"));
        }
        let palette = if raster_format & raster_format::PAL8 != 0 {
            Some(r.bytes(256 * 4)?.to_vec())
        } else {
            None
        };
        let mut levels = Vec::with_capacity(num_levels as usize);
        for _ in 0..num_levels {
            let size = r.u32()? as usize;
            levels.push(r.bytes(size)?.to_vec());
        }
        r.expect_end()?;
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(TextureNative {
            version: c.version,
            struct_version: s.version,
            platform,
            filter_addressing,
            name_raw,
            mask_raw,
            raster_format,
            format_word,
            width,
            height,
            depth,
            raster_type,
            flags_byte,
            palette,
            levels,
            extension,
        })
    }

    pub fn to_chunk(&self) -> Result<Chunk> {
        const CTX: &str = "RW texture native (write)";
        let pal8 = self.raster_format & raster_format::PAL8 != 0;
        if pal8 != self.palette.is_some() || self.palette.as_ref().is_some_and(|p| p.len() != 1024) {
            return Err(Error::invalid(CTX, "palette does not match the raster format"));
        }
        if self.levels.len() > 255 {
            return Err(Error::invalid(CTX, "more than 255 mip levels"));
        }
        let mut s = Vec::new();
        s.put_u32(self.platform);
        s.put_u32(self.filter_addressing);
        s.extend_from_slice(&self.name_raw);
        s.extend_from_slice(&self.mask_raw);
        s.put_u32(self.raster_format);
        s.put_u32(self.format_word);
        s.put_u16(self.width);
        s.put_u16(self.height);
        s.put_u8(self.depth);
        s.put_u8(self.levels.len() as u8);
        s.put_u8(self.raster_type);
        s.put_u8(self.flags_byte);
        if let Some(p) = &self.palette {
            s.extend_from_slice(p);
        }
        for l in &self.levels {
            s.put_u32(l.len() as u32);
            s.extend_from_slice(l);
        }
        Ok(Chunk::new_children(
            id::TEXTURENATIVE,
            self.version,
            vec![
                Chunk::new_data(id::STRUCT, self.struct_version, s),
                self.extension.to_chunk(),
            ],
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TexDictionary {
    pub version: u32,
    pub struct_version: u32,
    /// Second u16 of the struct (device id; values 0, 1, 2 and 6 occur in the install). Raw.
    pub device_id: u16,
    pub textures: Vec<TextureNative>,
    pub extension: Extension,
}

impl TexDictionary {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        const CTX: &str = "RW texture dictionary";
        let mut ch = Children::of(c, id::TEXDICTIONARY, CTX)?;
        let s = ch.next(id::STRUCT)?;
        let mut r = Reader::new(s.expect_data(id::STRUCT, CTX)?, CTX);
        let count = r.u16()?;
        let device_id = r.u16()?;
        r.expect_end()?;
        let mut textures = Vec::with_capacity(count as usize);
        for _ in 0..count {
            textures.push(TextureNative::from_chunk(ch.next(id::TEXTURENATIVE)?)?);
        }
        let extension = Extension::next(&mut ch)?;
        ch.finish()?;
        Ok(TexDictionary {
            version: c.version,
            struct_version: s.version,
            device_id,
            textures,
            extension,
        })
    }

    pub fn to_chunk(&self) -> Result<Chunk> {
        let count = u16::try_from(self.textures.len())
            .map_err(|_| Error::invalid("RW texture dictionary (write)", "more than 65535 textures"))?;
        let mut s = Vec::new();
        s.put_u16(count);
        s.put_u16(self.device_id);
        let mut children = vec![Chunk::new_data(id::STRUCT, self.struct_version, s)];
        for t in &self.textures {
            children.push(t.to_chunk()?);
        }
        children.push(self.extension.to_chunk());
        Ok(Chunk::new_children(id::TEXDICTIONARY, self.version, children))
    }

    pub fn find(&self, name: &str) -> Option<&TextureNative> {
        self.textures
            .iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rw::RwStream;

    const V: u32 = 0x1803FFFF;

    fn tex(name: &[u8], pal: bool) -> TextureNative {
        let mut name_raw = [0u8; 32];
        name_raw[..name.len()].copy_from_slice(name);
        name_raw[20] = 0xEE; // garbage after the NUL is preserved
        TextureNative {
            version: V,
            struct_version: V,
            platform: PLATFORM_D3D9,
            filter_addressing: 0x1106,
            name_raw,
            mask_raw: [0; 32],
            raster_format: if pal {
                raster_format::PAL8 | raster_format::C8888
            } else {
                raster_format::C565
            },
            format_word: if pal { 0 } else { D3DFMT_DXT1 },
            width: 4,
            height: 4,
            depth: if pal { 8 } else { 16 },
            raster_type: 4,
            flags_byte: if pal { 0 } else { 8 },
            palette: pal.then(|| vec![7; 1024]),
            levels: vec![vec![1; 8]],
            extension: Extension {
                version: V,
                plugins: vec![],
            },
        }
    }

    #[test]
    fn txd_round_trip() {
        let txd = TexDictionary {
            version: V,
            struct_version: V,
            device_id: 2,
            textures: vec![tex(b"road", false), tex(b"sky", true)],
            extension: Extension {
                version: V,
                plugins: vec![],
            },
        };
        let bytes = txd.to_chunk().unwrap().to_bytes();
        let st = RwStream::parse(&bytes).unwrap();
        let back = TexDictionary::from_chunk(&st.chunks[0]).unwrap();
        assert_eq!(back, txd);
        assert_eq!(back.to_chunk().unwrap().to_bytes(), bytes);
        assert_eq!(back.find("ROAD").unwrap().compression(), Compression::Dxt1);
        assert_eq!(back.find("sky").unwrap().palette.as_ref().unwrap().len(), 1024);
    }

    #[test]
    fn d3d8_compression_and_rejections() {
        let mut t = tex(b"a", false);
        t.platform = PLATFORM_D3D8;
        t.format_word = 1;
        t.flags_byte = 3;
        assert_eq!(t.compression(), Compression::Dxt3);
        assert!(t.has_alpha());
        let mut bad = tex(b"b", false);
        bad.raster_format |= raster_format::PAL4;
        let c = bad.to_chunk().unwrap();
        assert!(matches!(
            TextureNative::from_chunk(&c),
            Err(Error::Unsupported { .. })
        ));
        let mut bad = tex(b"c", false);
        bad.palette = Some(vec![0; 1024]);
        assert!(bad.to_chunk().is_err());
    }
}
