//! PORT-VEH-4 (batch `world4-1`, SA:MP-executed rows): `InteriorGroup_c::ArePathsLoaded`. Ported from the
//! 1.0 US bytes (gta-reversed does not reverse it).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, the result through `out`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CPathFind::AreNodesLoadedForArea(float minX, float maxX, float minY, float maxY)` (0x44DD10) on
    /// `ThePaths` (0x96F050); EAX as it leaves it
    fn are_nodes_loaded_for_area(paths: GPtr<u8>, min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> u32 = thiscall 0x44DD10 => unported;
}

/// `InteriorGroup_c::ArePathsLoaded()` (0x595380): the eight corners of the group entity's (+8)
/// col-model box (corner order: (x0, y0, z0), (x0, y1, z0), (x1, y1, z0), (x1, y0, z0), then the same
/// with z1) are transformed by the entity matrix (allocated and updated from the simple transform when
/// missing, re-checked per corner); their x/y bounds (starting at +/-999999) go to
/// `ThePaths.AreNodesLoadedForArea(minX, maxX, minY, maxY)`, whose EAX is returned unchanged.
#[hook(addr = 0x595380, conv = thiscall, name = "InteriorGroup_c::ArePathsLoaded", this = "InteriorGroup_c",
       expect = "81 EC 84 00 00 00 53 8B D9",
       unverified = "world4-1", route = "L2", hazards = "heap-writes",
       note = "may allocate the entity's matrix (matrix pool)")]
pub fn interior_group_are_paths_loaded(this: GPtr<u8>) -> u32 {
    let ent = rd::<GPtr<u8>>(this, 8);
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * i32::from(rd::<i16>(ent, 0x22)) as usize).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let lo = [rd::<u32>(col, 0), rd::<u32>(col, 4), rd::<u32>(col, 8)];
    let hi = [rd::<u32>(col, 0xC), rd::<u32>(col, 0x10), rd::<u32>(col, 0x14)];
    let mut corners = [
        [lo[0], lo[1], lo[2]],
        [lo[0], hi[1], lo[2]],
        [hi[0], hi[1], lo[2]],
        [hi[0], lo[1], lo[2]],
        [lo[0], lo[1], hi[2]],
        [lo[0], hi[1], hi[2]],
        [hi[0], hi[1], hi[2]],
        [hi[0], lo[1], hi[2]],
    ];
    let big = f32::from_bits(0x4974_23F0);
    let mut mn = [big; 3];
    let mut mx = [-big; 3];
    for c in corners.iter_mut() {
        let e = rd::<GPtr<u8>>(this, 8);
        if rd::<u32>(e, 0x14) == 0 {
            placeable_allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let mut out = [0u32; 3];
        let cp: GPtr<u8> = GPtr::from_raw(c as *mut [u32; 3]).cast();
        let r = matrix_mul_vec(GPtr::from_raw(&mut out as *mut [u32; 3]).cast(), rd::<GPtr<u8>>(e, 0x14), cp);
        for k in 0..3isize {
            wr::<u32>(cp, 4 * k, rd::<u32>(r, 4 * k));
        }
        for k in 0..3 {
            let v = f32::from_bits(c[k]);
            if v < mn[k] {
                mn[k] = v;
            }
        }
        for k in 0..3 {
            let v = f32::from_bits(c[k]);
            if v > mx[k] {
                mx[k] = v;
            }
        }
    }
    are_nodes_loaded_for_area(GPtr::from_addr(0x96_F050), mn[0], mx[0], mn[1], mx[1])
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERIOR_GROUP_ARE_PATHS_LOADED];
