//! FRONTIER-2 turn 2, lane C (batch `frontier2-c5`): RenderWare-side functions ported code still reached through the
//! original: the GTA helper `RpGeometryReplaceOldMaterialWithNewMaterial` and the D3D9 raster format setter. Ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{material_list_find_call, rp_material_destroy};
use crate::common::{g, rd, wr};

orig! {
    /// The D3D9 raster format resolver (0x4CC1D0, cdecl `(type, format)`; unnamed): the format flags (bits 8..15) the
    /// device supports for that raster type
    fn raster_resolve_format(ty: u32, format: u32) -> u32 = cdecl 0x4CC1D0 => unported;
}

/// `RpGeometryReplaceOldMaterialWithNewMaterial(RpGeometry*, RpMaterial* old, RpMaterial* new)` (0x734DE0, cdecl, AL):
/// over the geometry's mesh header (+0x54; mesh count word +4, meshes of 0xC from +0x10, material at +8) every mesh
/// whose material is `old`: its index in the geometry's material list (+0x20) (`_rpMaterialListFindMaterialIndex`),
/// `RpMaterialDestroy(old)`, the list slot and the mesh get `new`, `new`'s reference count (word +0x18) incremented;
/// AL = 1 when any mesh matched, else 0.
#[hook(addr = 0x734DE0, conv = cdecl, name = "RpGeometryReplaceOldMaterialWithNewMaterial", expect = "51 8B 44 24 08 53",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn rp_geometry_replace_old_material_with_new_material(geometry: GPtr<u8>, old: GPtr<u8>, new: GPtr<u8>) -> u8 {
    let header = rd::<GPtr<u8>>(geometry, 0x54);
    let mut n = i32::from(rd::<u16>(header, 4));
    let mut result = 0u8;
    if n <= 0 {
        return result;
    }
    let mut mesh = header.byte_add(0x18);
    loop {
        if rd::<GPtr<u8>>(mesh, 0) == old {
            let list = geometry.byte_add(0x20);
            let idx = material_list_find_call(list, rd::<GPtr<u8>>(mesh, 0));
            rp_material_destroy(rd::<GPtr<u8>>(mesh, 0));
            wr::<GPtr<u8>>(rd::<GPtr<u8>>(list, 0), idx.wrapping_mul(4) as isize, new);
            wr::<GPtr<u8>>(mesh, 0, new);
            wr::<u16>(new, 0x18, rd::<u16>(new, 0x18).wrapping_add(1));
            result = 1;
        }
        mesh = mesh.byte_add(0xC);
        n -= 1;
        if n == 0 {
            return result;
        }
    }
}

/// The D3D9 raster format setter (0x4CC580, cdecl `(raster, format)`; unnamed, called by `_rwD3D9ImageFindRasterFormat`):
/// type byte +0x20 = format & 7, flag byte +0x21 = format & 0xF8; `FUN_004cc1d0(type, format) >> 8` → format byte +0x23
/// and, through its low nibble, the depth +0x14 = the byte table 0x85C674[nibble * 8]. Returns 1.
#[hook(addr = 0x4CC580, conv = cdecl, name = "FUN_004cc580", expect = "8B 4C 24 08 8A C1",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn raster_set_format(raster: GPtr<u8>, format: u32) -> u32 {
    let ty = format as u8 & 7;
    wr::<u8>(raster, 0x20, ty);
    wr::<u8>(raster, 0x21, format as u8 & 0xF8);
    let f = raster_resolve_format(u32::from(ty), format) >> 8;
    wr::<u8>(raster, 0x23, f as u8);
    wr::<u32>(raster, 0x14, u32::from(g::<u8>(0x85C674 + 8 * (f & 0xF) as usize).read()));
    1
}

/// The D3D9 format check by raster type (0x4CC0C0, registers: cube flag in EAX, raster type in ECX, format in EDX;
/// unnamed, called by the format resolver 0x4CC1D0; FRONTIER-2 lane C turn 3): type `& 7` 0 (normal) or 4 (texture)
/// → `CheckDeviceFormat(usage 0, cube ? CUBETEXTURE : TEXTURE, format) >= 0`; 5 (camera texture) →
/// `_rwD3D9CheckValidCamera[Cube]TextureFormat(format)`; any other type → 0.
#[hook(addr = 0x4CC0C0, conv = reg("args = eax ecx edx"), name = "FUN_004cc0c0", expect = "56 8B F0 33 C0",
       unverified = "frontier2-c7", route = "L2", hazards = "none")]
pub fn raster_check_format_by_type(cube: u32, ty: u32, format: u32) -> u32 {
    match ty & 7 {
        0 | 4 => u32::from(crate::raster::check_format(0, if cube != 0 { 5 } else { 3 }, format) >= 0),
        5 if cube != 0 => crate::bindings::check_valid_camera_cube_texture_format(format),
        5 => crate::bindings::check_valid_camera_texture_format(format),
        _ => 0,
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] =
    &[HOOK_RP_GEOMETRY_REPLACE_OLD_MATERIAL_WITH_NEW_MATERIAL, HOOK_RASTER_SET_FORMAT, HOOK_RASTER_CHECK_FORMAT_BY_TYPE];
