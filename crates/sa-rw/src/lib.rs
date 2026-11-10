//! RenderWare object model for the phase B/C engine (PLAN §2, crate `sa-rw`).
//!
//! Built from the lossless stream types of `sa-formats` (`rw::dff`, `rw::txd`). The objects here
//! are plain Rust values (indices instead of pointers): a [`Clump`] owns its [`Frame`]s, [`Atomic`]s
//! and [`Geometry`]s, a [`TexDictionary`] owns its [`Texture`]s and their [`Raster`]s. Field names
//! follow the RW 3.6 structs in `docs/re/rw/objects.md` (`RpAtomic`, `RpGeometry`, `RpMaterial`,
//! `RwTexture`, `RwRaster`, ...) so the later 1:1 port can map them one to one; the in-memory
//! layouts are *not* the original ones yet (no `#[repr(C)]` mirrors, no plugin registry).
//!
//! The crate has no GPU and no OS dependency: it builds for `wasm32-unknown-unknown`.
//! [`dxt`] holds a software BC1/BC2/BC3 decoder used by renderers without BC texture support.
//!
//! What this crate interprets beyond the core objects (plugins, `docs/re/rw/plugins.md`):
//! * bin mesh (`0x50E`): the per-material index lists the game renders ([`Mesh`]);
//! * extra vertex colours (`0x253F2F9`, `CCustomBuildingDNPipeline`): night prelight colours;
//! * pipeline set (`0x253F2F3`): the custom pipeline id of an atomic;
//! * node name (`0x253F2FE`): frame names;
//! * 2d effects, collision, breakable, MatFX, reflection/specular materials, skin and HAnim are
//!   kept raw (not interpreted yet).

pub mod clump;
pub mod dxt;
pub mod geometry;
pub mod matrix;
pub mod texture;

pub use clump::{Atomic, Clump, Frame, read_clump};
pub use geometry::{Geometry, Material, MaterialTexture, Mesh};
pub use matrix::RwMatrix;
pub use texture::{Raster, RasterData, TexDictionary, Texture, read_tex_dictionary};

/// Errors of this crate: stream decoding errors from `sa-formats`, or a stream that lacks an
/// expected object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Format(sa_formats::Error),
    Missing(&'static str),
    Invalid(String),
}

impl From<sa_formats::Error> for Error {
    fn from(e: sa_formats::Error) -> Self {
        Error::Format(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Format(e) => write!(f, "{e}"),
            Error::Missing(what) => write!(f, "stream has no {what}"),
            Error::Invalid(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;

/// Plugin and pipeline ids interpreted here (values from `sa_formats::rw::id` and
/// `docs/re/rw/d3d9.md` §7).
pub mod ids {
    pub use sa_formats::rw::id::*;
    /// `CCustomBuildingPipeline` pipeline id (atomic pipeline plugin value).
    pub const PIPE_BUILDING: u32 = 0x53F2_009C;
    /// `CCustomBuildingDNPipeline` (day/night prelight blend).
    pub const PIPE_BUILDING_DN: u32 = 0x53F2_0098;
    /// `CCustomCarEnvMapPipeline`.
    pub const PIPE_VEHICLE: u32 = 0x53F2_009A;
}

/// Lower-cases ASCII bytes into a `String` (RW compares texture names case-insensitively).
pub fn lower_name(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    raw[..end].iter().map(|b| b.to_ascii_lowercase() as char).collect()
}

fn u32_at(d: &[u8], off: usize) -> Option<u32> {
    d.get(off..off + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
