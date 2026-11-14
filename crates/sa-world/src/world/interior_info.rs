//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `Interior_c` matrix, visibility, goto points
//! and interior info entries. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes
//! win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use super::common::{g, rd, wr, x, xi};

use crate::bindings::get_tile_centre as interior_get_tile_centre;
use crate::bindings::ig_get_entity;
use sa_core::bindings::rw_matrix_rotate;
use sa_core::bindings::rw_matrix_translate;
use crate::bindings::rw_v3d_transform_vectors;
use sa_core::bindings::rw_matrix_multiply;
use crate::bindings::get_tile_status as interior_get_tile_status;
use crate::bindings::is_pt_inside as interior_is_pt_inside;

/// `Interior_c::CalcMatrix(const CVector* pos)` (0x5914D0): the interior's RwMatrix (+0x18) is set to
/// identity (flags | 0x20003), rotated about z by the info's angle (+0x14 -> +0x20, replace),
/// translated by `pos` (post-concat) and multiplied by the group entity's frame matrix (`RwObject`
/// +0x18 -> parent frame +0x10; a null matrix without an RW object, as the original passes).
#[hook(addr = 0x5914D0, conv = thiscall, name = "Interior_c::CalcMatrix", this = "Interior_c",
       expect = "83 EC 0C 53 56",
       unverified = "world3-2", route = "L2", hazards = "x87-transcendental",
       note = "RwMatrixRotate uses sin/cos of the angle")]
pub fn interior_calc_matrix(this: GPtr<u8>, pos: GPtr<u8>) {
    let flags = rd::<u32>(this, 0x24) | 0x2_0003;
    for o in [0x40, 0x2C, 0x18] {
        wr::<u32>(this, o, 0x3F80_0000);
    }
    for o in [0x28, 0x20, 0x1C, 0x3C, 0x38, 0x30, 0x50, 0x4C, 0x48] {
        wr::<u32>(this, o, 0);
    }
    wr::<u32>(this, 0x24, flags);
    let mut axis = [0u32, 0, 0x3F80_0000];
    let m = this.byte_add(0x18);
    rw_matrix_rotate(m, GPtr::from_raw(&mut axis as *mut [u32; 3]).cast(), f32::from_bits(rd::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x20)), 0);
    rw_matrix_translate(m, pos, 2);
    let obj = rd::<GPtr<u8>>(ig_get_entity(rd::<GPtr<u8>>(this, 0xC)), 0x18);
    let b = if obj.is_null() { GPtr::from_addr(0) } else { rd::<GPtr<u8>>(obj, 4).byte_add(0x10) };
    rw_matrix_multiply(m, m, b);
}

/// `Interior_c::IsVisible()` (0x5929F0): the camera is inside the interior (5 x 5 margin) or, for
/// infos with a positive byte +5, within 10 units (2D) of the point +0x400.
#[hook(addr = 0x5929F0, conv = thiscall, name = "Interior_c::IsVisible", this = "Interior_c",
       expect = "A1 3C F0 B6 00",
       unverified = "world3-2", route = "L2", hazards = "none")]
pub fn interior_is_visible(this: GPtr<u8>) -> u8 {
    let cm = g::<GPtr<u8>>(0xB6_F03C).read();
    let cp = if cm.is_null() { GPtr::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
    let mut cam = [rd::<u32>(cp, 0), rd::<u32>(cp, 4), rd::<u32>(cp, 8)];
    if interior_is_pt_inside(this, GPtr::<u8>::cast::<f32>(GPtr::from_raw(&mut cam as *mut [u32; 3]).cast()), f32::from_bits(0x40A0_0000), f32::from_bits(0x40A0_0000), f32::from_bits(0)) != 0 {
        return 1;
    }
    if rd::<i8>(rd::<GPtr<u8>>(this, 0x14), 5) <= 0 {
        return 0;
    }
    let a = x(f32::from_bits(cam[0])).fsub_m(rd::<f32>(this, 0x400));
    let b = x(f32::from_bits(cam[1])).fsub_m(rd::<f32>(this, 0x404));
    let d2 = b.fmul(b).fadd(a.fmul(a));
    u8::from(!d2.fcom(x(100.0)).test_parity(0x05))
}

/// Tile (x, y) of the interior grid (+0x68, 30 per row).
fn tile(this: GPtr<u8>, tx: i32, ty: i32) -> GPtr<u8> {
    this.byte_add(0x68 + (tx as isize) * 0x1E + ty as isize)
}

/// `Interior_c::AddGotoPt(int32 x, int32 y, float dx, float dy)` (0x591D20): with fewer than 16 goto
/// points (+0x40C) and tile (x, y) free (status 3 inside the grid, else `GetTileStatus` 7), the tile
/// centre of (x + dx, y + dy) is stored with x/y bytes (+0x410 + 16k), and an in-range tile of
/// status 0 or 3 becomes 4.
#[hook(addr = 0x591D20, conv = thiscall, name = "Interior_c::AddGotoPt", this = "Interior_c",
       expect = "83 EC 0C 56 8B F1",
       unverified = "world3-2", route = "L2", hazards = "none")]
pub fn interior_add_goto_pt(this: GPtr<u8>, tx: i32, ty: i32, dx: f32, dy: f32) {
    if rd::<i8>(this, 0x40C) >= 0x10 {
        return;
    }
    let info = rd::<GPtr<u8>>(this, 0x14);
    let inside = tx < i32::from(rd::<u8>(info, 2)) && ty < i32::from(rd::<u8>(info, 3)) && tx >= 0 && ty >= 0;
    if !(inside && tile(this, tx, ty).read() == 3) && interior_get_tile_status(this, tx, ty) as i32 != 7 {
        return;
    }
    let mut p = [0u32; 3];
    interior_get_tile_centre(
        this,
        xi(tx).fadd_m(dx).to_f32(),
        xi(ty).fadd_m(dy).to_f32(),
        GPtr::from_raw(&mut p as *mut [u32; 3]).cast(),
    );
    let k = rd::<i8>(this, 0x40C) as isize;
    for (j, v) in p.iter().enumerate() {
        wr::<u32>(this, 0x414 + k * 16 + 4 * j as isize, *v);
    }
    wr::<u8>(this, 0x410 + (rd::<i8>(this, 0x40C) as isize) * 16, tx as u8);
    wr::<u8>(this, 0x411 + (rd::<i8>(this, 0x40C) as isize) * 16, ty as u8);
    if tx >= 0 && ty >= 0 {
        let info = rd::<GPtr<u8>>(this, 0x14);
        // `lea edi, [x + 1]; cmp edi, w; jg`: x + 1 <= w (the wrap at x = i32::MAX is not modelled)
        if tx < i32::from(rd::<u8>(info, 2)) && ty < i32::from(rd::<u8>(info, 3)) {
            let t = tile(this, tx, ty);
            let s = t.read();
            if s == 3 || s == 0 {
                t.write(4);
            }
        }
    }
    wr::<i8>(this, 0x40C, rd::<i8>(this, 0x40C).wrapping_add(1));
}

/// `Interior_c::AddInteriorInfo(int32 kind, float x, float y, int32 dir, CEntity*)` (0x591E40): with
/// fewer than 16 infos (+0x40D) an entry (+0x590 + 32k): kind byte, the tile centre of (x, y) raised
/// by 0.8, the direction (dir 0..3 = -y, +x, +y, -x; other values zero; -1 untransformed, else
/// through the interior matrix +0x18), byte +1 = 0, the entity (+0x1C); true. False when full.
#[hook(addr = 0x591E40, conv = thiscall, name = "Interior_c::AddInteriorInfo", this = "Interior_c",
       expect = "83 EC 18 56 8B F1",
       unverified = "world3-2", route = "L2", hazards = "none")]
pub fn interior_add_interior_info(this: GPtr<u8>, kind: u8, px: f32, py: f32, dir: i32, ent: u32) -> u8 {
    if rd::<i8>(this, 0x40D) >= 0x10 {
        return 0;
    }
    let mut p = [0f32; 3];
    interior_get_tile_centre(this, px, py, GPtr::from_raw(&mut p as *mut [f32; 3]).cast());
    p[2] = x(p[2]).fadd_m(f32::from_bits(0x3F4C_CCCD)).to_f32();
    let mut d = [0u32; 3];
    if dir != -1 {
        match dir {
            3 => d[0] = 0xBF80_0000,
            1 => d[0] = 0x3F80_0000,
            2 => d[1] = 0x3F80_0000,
            0 => d[1] = 0xBF80_0000,
            _ => {}
        }
        let dp: GPtr<u8> = GPtr::from_raw(&mut d as *mut [u32; 3]).cast();
        rw_v3d_transform_vectors(dp, dp, 1, this.byte_add(0x18));
    }
    let k = || rd::<i8>(this, 0x40D) as isize * 0x20;
    wr::<u8>(this, 0x590 + k(), kind);
    for (j, v) in p.iter().enumerate() {
        wr::<f32>(this, 0x594 + k() + 4 * j as isize, *v);
    }
    for (j, v) in d.iter().enumerate() {
        wr::<u32>(this, 0x5A0 + k() + 4 * j as isize, *v);
    }
    wr::<u8>(this, 0x591 + k(), 0);
    wr::<u32>(this, 0x5AC + k(), ent);
    wr::<i8>(this, 0x40D, rd::<i8>(this, 0x40D).wrapping_add(1));
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTERIOR_CALC_MATRIX,
    HOOK_INTERIOR_IS_VISIBLE,
    HOOK_INTERIOR_ADD_GOTO_PT,
    HOOK_INTERIOR_ADD_INTERIOR_INFO,
];
