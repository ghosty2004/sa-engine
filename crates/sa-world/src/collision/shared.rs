//! Collision layouts shared by the COLL modules (gta-reversed `Collision/*.h`, offsets checked against
//! the bytes; sizes asserted against `sa_core::layouts::size` on 32-bit targets).

use sa_core::layouts::size;
use sa_core::mem::{GPtr, GameData};
use sa_math::vector::CVector;

/// `fld`-able f32 constants of `.rdata` used by the COLL code (bits).
pub mod k {
    /// 0x858B50: 0.0
    pub const F0: u32 = 0x0000_0000;
    /// 0x858624: 1.0
    pub const F1: u32 = 0x3F80_0000;
    /// 0x858B8C: 0.5
    pub const F0_5: u32 = 0x3F00_0000;
    /// 0x858B88: 1/128 (compressed vertex and plane distance scale)
    pub const F1_128: u32 = 0x3C00_0000;
    /// 0x858BFC: 1/4096 (compressed plane normal scale)
    pub const F1_4096: u32 = 0x3980_0000;
    /// 0x858BF8: 4096.0
    pub const F4096: u32 = 0x4580_0000;
    /// 0x858BF4: 128.0
    pub const F128: u32 = 0x4300_0000;
    /// 0x858C14: 9.99999975e-6
    pub const SMALL: u32 = 0x3727_C5AC;
}

/// `f32` from a `.rdata` constant.
pub fn cf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// A byte pointer to field `$f` of the struct `$T` that `$p` points at (`.cast::<F>()` it).
#[macro_export]
macro_rules! field {
    ($p:expr, $T:ty, $f:ident) => {
        $p.cast::<u8>().byte_add(core::mem::offset_of!($T, $f) as isize)
    };
}

/// `CompressedVector` (6 bytes): int16 x, y, z, scaled by 1/128.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompressedVector {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

/// `CColTriangle` (8 bytes): vertex indices, material, light.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CColTriangle {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub material: u8,
    pub light: u8,
}

/// `CColTrianglePlane` (10 bytes): int16 normal (1/4096), int16 distance (1/128), orientation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CColTrianglePlane {
    pub nx: i16,
    pub ny: i16,
    pub nz: i16,
    pub dist: i16,
    pub orientation: u8,
    pub pad: u8,
}

/// `CCollisionData` (0x30).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CCollisionData {
    pub num_spheres: u16,
    pub num_boxes: u16,
    pub num_triangles: u16,
    pub num_lines: u8,
    /// bit 0 `bUsesDisks`, bit 1 `bNotEmpty`, bit 2 `bHasShadowInfo`, ...
    pub flags: u8,
    pub spheres: GPtr<u8>,
    pub boxes: GPtr<u8>,
    pub lines: GPtr<u8>,
    pub vertices: GPtr<CompressedVector>,
    pub triangles: GPtr<CColTriangle>,
    pub triangle_planes: GPtr<CColTrianglePlane>,
    pub num_shadow_triangles: u32,
    pub num_shadow_vertices: u32,
    pub shadow_vertices: GPtr<CompressedVector>,
    pub shadow_triangles: GPtr<CColTriangle>,
}

/// `CColModel` (0x30).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CColModel {
    /// `CBoundingBox` (min, max)
    pub bbox: [f32; 6],
    /// `CSphere` (centre, radius)
    pub sphere: [f32; 4],
    pub col_slot: u8,
    /// bit 1 `m_bIsSingleColDataAlloc`
    pub flags: u8,
    pub pad: [u8; 2],
    pub col_data: GPtr<CCollisionData>,
}

/// `CLink<CCollisionData*>` (12 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CLink {
    pub data: GPtr<CCollisionData>,
    pub prev: GPtr<CLink>,
    pub next: GPtr<CLink>,
}

/// `CLinkList<CCollisionData*>` (0x34).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CLinkList {
    pub used_head: CLink,
    pub used_tail: CLink,
    pub free_head: CLink,
    pub free_tail: CLink,
    pub links: GPtr<CLink>,
}

/// `CColPoint` (0x2C), as raw words (fields through offsets: point +0, normal +0x10, surfaces
/// +0x20..+0x25, depth +0x28).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CColPoint {
    pub point: CVector,
    pub pad0: f32,
    pub normal: CVector,
    pub pad1: f32,
    pub surface_a: u8,
    pub piece_a: u8,
    pub lighting_a: u8,
    pub surface_b: u8,
    pub piece_b: u8,
    pub lighting_b: u8,
    pub pad2: [u8; 2],
    pub depth: f32,
}

// SAFETY: plain data (integers, floats, game pointers).
#[allow(unsafe_code)]
unsafe impl GameData for CompressedVector {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CColTriangle {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CColTrianglePlane {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CCollisionData {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CColModel {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CLink {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CLinkList {}
// SAFETY: as above.
#[allow(unsafe_code)]
unsafe impl GameData for CColPoint {}

const _: () = {
    assert!(core::mem::size_of::<CompressedVector>() == 6);
    assert!(core::mem::size_of::<CColTriangle>() == size::CColTriangle);
    assert!(core::mem::size_of::<CColTrianglePlane>() == size::CColTrianglePlane);
    assert!(core::mem::size_of::<CColPoint>() == size::CColPoint);
    assert!(core::mem::offset_of!(CColPoint, surface_a) == 0x20);
    assert!(core::mem::offset_of!(CColPoint, depth) == 0x28);
};

#[cfg(target_pointer_width = "32")]
const _: () = {
    assert!(core::mem::size_of::<CCollisionData>() == size::CCollisionData);
    assert!(core::mem::offset_of!(CCollisionData, vertices) == 0x14);
    assert!(core::mem::offset_of!(CCollisionData, triangle_planes) == 0x1C);
    assert!(core::mem::offset_of!(CCollisionData, shadow_vertices) == 0x28);
    assert!(core::mem::size_of::<CColModel>() == size::CColModel);
    assert!(core::mem::offset_of!(CColModel, flags) == 0x29);
    assert!(core::mem::offset_of!(CColModel, col_data) == 0x2C);
    assert!(core::mem::size_of::<CLinkList>() == size::CLinkList_void);
    assert!(core::mem::offset_of!(CLinkList, links) == 0x30);
};

/// Reads a `CVector` from game memory (three f32 words; `CVector` lives in sa-math, which has no
/// `GameData` impl).
pub fn read_vec(p: GPtr<CVector>) -> CVector {
    let f = p.cast::<f32>();
    CVector::new(f.read(), f.wrapping_index(1).read(), f.wrapping_index(2).read())
}

/// Writes a `CVector` (x, y, z in that order).
pub fn write_vec(p: GPtr<CVector>, v: CVector) {
    let f = p.cast::<f32>();
    f.write(v.x);
    f.wrapping_index(1).write(v.y);
    f.wrapping_index(2).write(v.z);
}
