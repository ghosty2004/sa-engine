//! WRITER-WORLD-2 (batch `world6-4`): `Interior_c::GetBoundingBox`. Ported from the 1.0 US bytes;
//! gta-reversed has it unreversed.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, xi};

use crate::bindings::get_tile_centre;
orig! {
    /// `Interior_c::FindBoundingBox(int32 x, int32 y, int32* minX, int32* maxX, int32* minY, int32* maxY, int32* visited)` (0x5922C0)
    #[allow(clippy::too_many_arguments)]
    fn find_bounding_box(this: GPtr<u8>, tx: i32, ty: i32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, grid: GPtr<u8>) = thiscall 0x5922C0 => unported;
}

fn p(v: &mut i32) -> GPtr<u8> {
    GPtr::from_raw(v as *mut i32).cast()
}

/// `Interior_c::GetBoundingBox(FurnitureEntity_c* item, CVector* corners)` (0x593DB0): only for the
/// interior types 0, 1 and 6 (else false). The tile block of the item's tile (+0xC, +0xE) is grown
/// by `FindBoundingBox` over a cleared 30 x 30 visited grid; the four corners (tile centres of the
/// block edges pushed out by the margin at 0x8D22B0 plus half a tile) go to `corners[0..4]`: (minX,
/// maxY), (minX, minY), (maxX, minY), (maxX, maxY). Returns true.
#[hook(addr = 0x593DB0, conv = thiscall, name = "Interior_c::GetBoundingBox", this = "Interior_c",
       expect = "55 8B EC 83 E4 F8",
       unverified = "world6-4", route = "L2", hazards = "none",
       note = "0x3840-byte visited grid on the stack")]
pub fn get_bounding_box(this: GPtr<u8>, item: GPtr<u8>, corners: GPtr<u8>) -> u8 {
    let ty = rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0);
    if ty != 0 && ty != 1 && ty != 6 {
        return 0;
    }
    let tx = i32::from(rd::<u16>(item, 0xC));
    let tyy = i32::from(rd::<u16>(item, 0xE));
    let mut grid = [0i32; 900];
    grid[(tx * 30 + tyy) as usize] = 1;
    let (mut a, mut b, mut c, mut d) = (tx, tx, tyy, tyy);
    find_bounding_box(this, tx, tyy, p(&mut a), p(&mut b), p(&mut c), p(&mut d),
                      GPtr::from_raw(&mut grid as *mut [i32; 900]).cast());
    let m = g::<f32>(0x8D_22B0).read();
    let lo = |v: i32| xi(v).fsub_m(0.5).fsub_m(m).to_f32();
    let hi = |v: i32| xi(v).fadd_m(m).fadd_m(0.5).to_f32();
    get_tile_centre(this, lo(a), hi(d), corners);
    get_tile_centre(this, lo(a), lo(c), corners.byte_add(0xC));
    get_tile_centre(this, hi(b), lo(c), corners.byte_add(0x18));
    get_tile_centre(this, hi(b), hi(d), corners.byte_add(0x24));
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GET_BOUNDING_BOX];
