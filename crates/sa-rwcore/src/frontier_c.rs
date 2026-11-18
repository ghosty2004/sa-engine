//! FRONTIER-2 turn 2, lane C (batch `frontier2-c5`): RenderWare-side functions ported code still reached through the
//! original: the GTA helper `RpGeometryReplaceOldMaterialWithNewMaterial` and the D3D9 raster format setter. Ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use crate::bindings::{material_list_find_call, rp_material_destroy};
use crate::common::{g, rd, wr};
use crate::bindings::check_valid_texture_format_call as texture_valid;

use crate::raster::{d3d9_check_valid_camera_texture_format as camera_ok, d3d9_check_valid_texture_format as texture_ok};

// The low byte of `D3DCAPS9.TextureCaps`; bit 7 (D3DPTEXTURECAPS_ALPHAPALETTE, read as a sign test) gives PAL8 an
// 8888 palette, else 888.
use crate::d3d9state::CAPS_TEXTURE as CAPS_TEXTURE_BYTE;
/// The display mode depth (0xC9BEF4) and the Z-buffer depth (0xC9BEFC).
const MODE_DEPTH: usize = 0xC9BEF4;
const Z_DEPTH: usize = 0xC9BEFC;

/// Raster format nibbles (bits 8..11 of the RW raster format): 0x100 1555, 0x200 565, 0x300 4444, 0x500 8888, 0x600
/// 888, 0x400 LUM8; 0x2000 PAL8, 0x4000 PAL4 (made PAL8).
const NIBBLE: u32 = 0xF00;

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

/// The D3D9 raster format resolver (0x4CC1D0, cdecl `(type, format)`; unnamed, called by the format setter 0x4CC580):
/// the format bits 8..15 the device supports for a raster of that type, from `format & 0xFF00`. Types 0 / 4 (normal /
/// texture) fall back between 8888 (D3DFMT 0x15), 888 (0x16), 4444 (0x1A), 565 / 1555 and PAL8 (0x29) through
/// `_rwD3D9CheckValidTextureFormat`, LUM8 (0x32) becoming the 888 fallback `| 0x16` as the bytes do; with no nibble the
/// display depth (0xC9BEF4) > 16 prefers 32-bit formats. Type 5 (camera texture) matches the Z-buffer depth (0xC9BEFC)
/// with `_rwD3D9CheckValidCameraTextureFormat`. Type 2 (Z-buffer) without a nibble takes the word at 0xB4E7E2 +
/// [0xC9C048] * 4, type 1 (camera) 0x700 / 0x900 for a 16 / 32-bit Z-buffer. Other types return the input bits.
#[hook(addr = 0x4CC1D0, conv = cdecl, name = "FUN_004cc1d0", expect = "8B 44 24 04 56 8B 74 24 0C",
       unverified = "frontier2-c13", route = "L2", hazards = "none")]
pub fn raster_resolve_format(ty: u32, format: u32) -> u32 {
    let mut f = format & 0xFF00;
    match ty {
        0 | 4 => resolve_texture(f),
        5 => {
            let nib = f & NIBBLE;
            let zd = g::<u32>(Z_DEPTH).read();
            if nib == 0 {
                return if zd != 0x10 && camera_ok(0x16) != 0 { f | 0x600 } else { f | 0x200 };
            }
            let bpp = u32::from(g::<u8>(0x85C674 + 8 * ((f >> 8) & 0xF) as usize).read());
            if zd == 0x10 {
                if bpp == 0x20 {
                    f = match nib {
                        0x500 => (f & !0x400) | 0x300,
                        0x600 => (f & !0x400) | 0x200,
                        _ => 0x200,
                    };
                }
                if f & NIBBLE == 0x300 && camera_ok(0x1A) == 0 {
                    f = (f & !0x200) | 0x100;
                }
                if f & NIBBLE != 0x100 || camera_ok(0x19) != 0 {
                    return f;
                }
                (f & !0x100) | 0x200
            } else {
                if bpp == 0x10 {
                    f = match nib {
                        0x300 => (f & !0x200) | 0x500,
                        0x100 => f | 0x500,
                        0x200 => f | 0x600,
                        _ => 0x600,
                    };
                }
                if f & NIBBLE != 0x500 || camera_ok(0x15) != 0 {
                    return f;
                }
                (f & !0x100) | 0x600
            }
        }
        2 if f & NIBBLE == 0 => u32::from(g::<u16>((0xB4_E7E2u32).wrapping_add(g::<u32>(0xC9C048).read().wrapping_mul(4)) as usize).read()),
        1 if f & NIBBLE == 0 => match g::<u32>(Z_DEPTH).read() {
            0x10 => 0x700,
            0x20 => 0x900,
            _ => f,
        },
        _ => f,
    }
}

/// The type 0 / 4 arm of 0x4CC1D0 (texture formats through `_rwD3D9CheckValidTextureFormat`).
fn resolve_texture(mut f: u32) -> u32 {
    // the shared fallback tails: 4444 else 1555 (0x4CC320), 565 (0x4CC351)
    let to_4444 = |f: u32| if texture_ok(0x1A) != 0 { f | 0x300 } else { f | 0x100 };
    let to_565 = |f: u32| (f & !0x400) | 0x200;
    if f & NIBBLE == 0 {
        if f & 0x2000 != 0 {
            if texture_ok(0x29) != 0 {
                f |= if (g::<u8>(CAPS_TEXTURE_BYTE).read() as i8) < 0 { 0x500 } else { 0x600 };
            } else {
                f &= !0x2000;
            }
            if f & 0x2000 != 0 {
                return f;
            }
        }
        if texture_ok(0x15) != 0 {
            return f | 0x500;
        }
        return to_4444(f);
    }
    if f & 0x4000 != 0 {
        f = (f & !0x4000) | 0x2000;
    }
    if f & 0x2000 != 0 {
        if texture_ok(0x29) != 0 && (f & NIBBLE != 0x500 || (g::<u8>(CAPS_TEXTURE_BYTE).read() as i8) < 0) {
            return f;
        }
        let depth = g::<i32>(MODE_DEPTH).read();
        f &= !0x2000;
        if f & NIBBLE == 0x600 {
            if depth > 0x10 && texture_ok(0x16) != 0 {
                return f | 0x600;
            }
            return to_565(f);
        }
        if depth > 0x10 && texture_ok(0x15) != 0 {
            return f | 0x500;
        }
        return to_4444(f & !0x500);
    }
    match f & NIBBLE {
        0x500 => {
            if texture_ok(0x15) != 0 {
                f
            } else {
                to_4444(f & !0x500)
            }
        }
        0x600 => {
            if texture_ok(0x16) != 0 {
                f
            } else {
                to_565(f)
            }
        }
        0x300 => {
            if texture_ok(0x1A) != 0 {
                f
            } else {
                (f & !0x200) | 0x100
            }
        }
        0x400 => {
            if texture_ok(0x32) != 0 {
                return f;
            }
            f &= !0x400;
            if texture_ok(0x16) != 0 {
                f | 0x16
            } else {
                f | 0x200
            }
        }
        _ => f,
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

/// `_rwD3D9ImageFindFormat(image)` (0x7FFFF0): the raster format for an image. 4 / 8-bit: all palette alphas 0xFF
/// (16 / 256 entries), or every pixel's palette alpha 0xFF (rows of `width` bytes), gives PAL4 / PAL8 | 888 (0x4600 /
/// 0x2600), else PAL4 / PAL8 | 8888 (0x4500 / 0x2500). Otherwise (32-bit) the pixel alphas: all 0xFF gives 888 (0x600)
/// when the display depth is above 16 and D3DFMT_X8R8G8B8 is valid, else 565 (0x200); any alpha 0x10..0xFE gives 8888
/// (0x500) when the display depth is above 16 and D3DFMT_A8R8G8B8 is valid, else 4444 (0x300) when D3DFMT_A4R4G4B4
/// is valid, else 1555 (0x100); only alphas 0..0xF besides 0xFF give 8888 (above 16 bits, A8R8G8B8 valid) or 1555.
/// The original keeps its row counter in its own argument slot.
#[hook(addr = 0x7FFFF0, conv = cdecl, name = "_rwD3D9ImageFindFormat", expect = "83 EC 0C 53 8B 5C 24 14",
       unverified = "frontier2-c23", route = "L2", hazards = "stack-residue",
       note = "the 32-bit path writes its row counter into its image argument slot")]
pub fn d3d9_image_find_format(img: GPtr<u8>) -> u32 {
    let depth: i32 = rd(img, 0xC);
    let deep = || g::<i32>(MODE_DEPTH).read() > 16;
    if depth == 4 || depth == 8 {
        let w: i32 = rd(img, 4);
        let pal: GPtr<u8> = rd(img, 0x18);
        let h: i32 = rd(img, 8);
        let n = if depth == 4 { 0x10 } else { 0x100 };
        if (0..n).any(|i| rd::<u8>(pal, i * 4 + 3) != 0xFF) {
            let mut row: GPtr<u8> = rd(img, 0x14);
            for _ in 0..h.max(0) {
                for x in 0..w.max(0) as isize {
                    if rd::<u8>(pal, isize::from(rd::<u8>(row, x)) * 4 + 3) != 0xFF {
                        return if depth == 4 { 0x4500 } else { 0x2500 };
                    }
                }
                row = row.byte_add(rd::<i32>(img, 0x10) as isize);
            }
        }
        return if depth == 4 { 0x4600 } else { 0x2600 };
    }
    let w: i32 = rd(img, 4);
    let mut row: GPtr<u8> = rd(img, 0x14);
    let mut low = false;
    let mut y = 0i32;
    while y < rd::<i32>(img, 8) {
        for x in 0..w.max(0) as isize {
            let a = rd::<u8>(row, x * 4 + 3);
            if a == 0xFF {
                continue;
            }
            if a > 0xF {
                if deep() && texture_valid(0x15) != 0 {
                    return 0x500;
                }
                return if texture_valid(0x1A) != 0 { 0x300 } else { 0x100 };
            }
            low = true;
        }
        row = row.byte_add(rd::<i32>(img, 0x10) as isize);
        y += 1;
    }
    match (low, deep()) {
        (false, true) => if texture_valid(0x16) != 0 { 0x600 } else { 0x200 },
        (false, false) => 0x200,
        (true, true) => if texture_valid(0x15) != 0 { 0x500 } else { 0x100 },
        (true, false) => 0x100,
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] =
    &[
        HOOK_D3D9_IMAGE_FIND_FORMAT,
        HOOK_RP_GEOMETRY_REPLACE_OLD_MATERIAL_WITH_NEW_MATERIAL,
        HOOK_RASTER_SET_FORMAT,
        HOOK_RASTER_CHECK_FORMAT_BY_TYPE,
        HOOK_RASTER_RESOLVE_FORMAT,
    ];
