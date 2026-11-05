//! RenderWare binary streams (DFF, TXD): generic chunk tree plus typed decoders.
//!
//! The chunk tree ([`chunk`]) is the lossless representation. The typed decoders ([`dff`], [`txd`])
//! read from a chunk tree and can rebuild an identical tree (`to_chunk`), so a typed round trip is
//! byte-exact as well. Plugin chunks inside extensions stay raw ([`Extension`]).

pub mod chunk;
pub mod dff;
pub mod txd;

pub use chunk::{Chunk, ChunkBody, ChunkHeader, RwStream};

use crate::bytes::c_str;
use crate::error::Result;
use chunk::Children;

/// Chunk type IDs. Core IDs from gta-reversed `rwplcore.h` (`rwVENDORID_CORE`), toolkit IDs from
/// `rpcriter.h` (`rwVENDORID_CRITERIONTK` = 1), world IDs from `rpworld.h`, Rockstar IDs from the
/// gta-reversed plugin headers (`rwVENDORID_DEVELOPER` = 0x0253F2).
pub mod id {
    pub const NAOBJECT: u32 = 0x00;
    pub const STRUCT: u32 = 0x01;
    pub const STRING: u32 = 0x02;
    pub const EXTENSION: u32 = 0x03;
    pub const CAMERA: u32 = 0x05;
    pub const TEXTURE: u32 = 0x06;
    pub const MATERIAL: u32 = 0x07;
    pub const MATLIST: u32 = 0x08;
    pub const WORLD: u32 = 0x0B;
    pub const FRAMELIST: u32 = 0x0E;
    pub const GEOMETRY: u32 = 0x0F;
    pub const CLUMP: u32 = 0x10;
    pub const LIGHT: u32 = 0x12;
    pub const ATOMIC: u32 = 0x14;
    pub const TEXTURENATIVE: u32 = 0x15;
    pub const TEXDICTIONARY: u32 = 0x16;
    pub const GEOMETRYLIST: u32 = 0x1A;
    /// `rwID_ANIMANIMATION` / `rwID_HANIMANIMATION`.
    pub const ANIMANIMATION: u32 = 0x1B;
    pub const RIGHTTORENDER: u32 = 0x1F;
    /// UV animation dictionary (top-level in some SA DFFs). Payload: Struct + 0x1B animations.
    pub const UVANIMDICT: u32 = 0x2B;
    /// `rwID_SKYMIPMAPVAL` (texture extension).
    pub const SKYMIPMAPVAL: u32 = 0x110;
    pub const SKINPLUGIN: u32 = 0x116;
    pub const HANIMPLUGIN: u32 = 0x11E;
    pub const MATERIALEFFECTSPLUGIN: u32 = 0x120;
    /// `rwID_ADCPLUGIN`.
    pub const ADCPLUGIN: u32 = 0x134;
    pub const UVANIMPLUGIN: u32 = 0x135;
    /// `rwID_BINMESHPLUGIN` (`rwVENDORID_CRITERIONWORLD`, 0x0E).
    pub const BINMESHPLUGIN: u32 = 0x50E;
    pub const NATIVEDATAPLUGIN: u32 = 0x510;
    pub const PIPELINEPLUGIN: u32 = 0x0253_F2F3;
    pub const ENVMAPATMPLUGIN: u32 = 0x0253_F2F4;
    pub const SPECMAPPLUGIN: u32 = 0x0253_F2F6;
    pub const TWODEFFECTPLUGIN: u32 = 0x0253_F2F8;
    pub const EXTRAVERTCOLOURPLUGIN: u32 = 0x0253_F2F9;
    pub const COLLISIONPLUGIN: u32 = 0x0253_F2FA;
    pub const ENVMAPPLUGIN: u32 = 0x0253_F2FC;
    pub const BREAKABLEPLUGIN: u32 = 0x0253_F2FD;
    pub const NODENAMEPLUGIN: u32 = 0x0253_F2FE;

    /// Human-readable name for reports.
    pub fn name(ty: u32) -> &'static str {
        match ty {
            NAOBJECT => "NAObject",
            STRUCT => "Struct",
            STRING => "String",
            EXTENSION => "Extension",
            CAMERA => "Camera",
            TEXTURE => "Texture",
            MATERIAL => "Material",
            MATLIST => "MaterialList",
            WORLD => "World",
            FRAMELIST => "FrameList",
            GEOMETRY => "Geometry",
            CLUMP => "Clump",
            LIGHT => "Light",
            ATOMIC => "Atomic",
            TEXTURENATIVE => "TextureNative",
            TEXDICTIONARY => "TexDictionary",
            GEOMETRYLIST => "GeometryList",
            ANIMANIMATION => "AnimAnimation",
            RIGHTTORENDER => "RightToRender",
            UVANIMDICT => "UVAnimDictionary",
            SKYMIPMAPVAL => "SkyMipmapVal",
            SKINPLUGIN => "Skin",
            HANIMPLUGIN => "HAnim",
            MATERIALEFFECTSPLUGIN => "MatFX",
            ADCPLUGIN => "ADC",
            UVANIMPLUGIN => "UVAnim",
            BINMESHPLUGIN => "BinMesh",
            NATIVEDATAPLUGIN => "NativeData",
            PIPELINEPLUGIN => "R*Pipeline",
            ENVMAPATMPLUGIN => "R*EnvMapAtomic",
            SPECMAPPLUGIN => "R*SpecMap",
            TWODEFFECTPLUGIN => "R*2dEffect",
            EXTRAVERTCOLOURPLUGIN => "R*NightVertexColours",
            COLLISIONPLUGIN => "R*Collision",
            ENVMAPPLUGIN => "R*EnvMap(reflection)",
            BREAKABLEPLUGIN => "R*Breakable",
            NODENAMEPLUGIN => "R*NodeName",
            _ => "unknown",
        }
    }
}

/// A library version stamp from a chunk header.
///
/// Unpacking follows librw `libraryIDUnpackVersion`/`libraryIDUnpackBuild` (matches the stamps in
/// the install: 0x1803FFFF = 3.6.0.3, 0x1400FFFF = 3.5.0.0, 0x1003FFFF = 3.4.0.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryStamp(pub u32);

impl LibraryStamp {
    /// Version as `0x3XYZZ`-style number (e.g. 0x36003), the form compared against in RW code.
    pub fn version(self) -> u32 {
        let v = self.0;
        if v & 0xFFFF_0000 != 0 {
            (((v >> 14) & 0x3FF00) + 0x30000) | ((v >> 16) & 0x3F)
        } else {
            v << 8
        }
    }

    /// Build number (0xFFFF for release builds).
    pub fn build(self) -> u32 {
        if self.0 & 0xFFFF_0000 != 0 {
            self.0 & 0xFFFF
        } else {
            0
        }
    }
}

/// An Extension chunk: plugin chunks kept raw.
#[derive(Debug, Clone, PartialEq)]
pub struct Extension {
    pub version: u32,
    pub plugins: Vec<Chunk>,
}

impl Extension {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        Ok(Extension {
            version: c.version,
            plugins: c.expect_children(id::EXTENSION, "RW extension")?.to_vec(),
        })
    }

    pub fn to_chunk(&self) -> Chunk {
        Chunk::new_children(id::EXTENSION, self.version, self.plugins.clone())
    }

    /// First plugin chunk of the given type.
    pub fn find(&self, ty: u32) -> Option<&Chunk> {
        self.plugins.iter().find(|c| c.ty == ty)
    }

    pub(crate) fn next(children: &mut Children<'_>) -> Result<Self> {
        Self::from_chunk(children.next(id::EXTENSION)?)
    }
}

/// A String chunk (NUL-terminated, padded to 4 bytes; the padding is kept raw).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwString {
    pub version: u32,
    pub raw: Vec<u8>,
}

impl RwString {
    pub fn from_chunk(c: &Chunk) -> Result<Self> {
        Ok(RwString {
            version: c.version,
            raw: c.expect_data(id::STRING, "RW string")?.to_vec(),
        })
    }

    pub fn to_chunk(&self) -> Chunk {
        Chunk::new_data(id::STRING, self.version, self.raw.clone())
    }

    /// Bytes up to the first NUL.
    pub fn as_bytes(&self) -> &[u8] {
        c_str(&self.raw)
    }

    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self.as_bytes()).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_stamps() {
        assert_eq!(LibraryStamp(0x1803FFFF).version(), 0x36003);
        assert_eq!(LibraryStamp(0x1803FFFF).build(), 0xFFFF);
        assert_eq!(LibraryStamp(0x1400FFFF).version(), 0x35000);
        assert_eq!(LibraryStamp(0x1003FFFF).version(), 0x34003);
        assert_eq!(LibraryStamp(0x310).version(), 0x31000);
        assert_eq!(LibraryStamp(0x310).build(), 0);
    }
}
