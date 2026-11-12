//! WRITER-WORLD (batch `world5-1`, SA:MP-executed rows): `InteriorGroup_c::ArePathsLoaded`. Ported
//! from the 1.0 US bytes (gta-reversed has only a `plugin::Call` stub; spec
//! `docs/re/specs/WORLD/InteriorGroup_c.md`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, x};

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890): the result through the hidden pointer
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CPathFind::AreNodesLoadedForArea(float minX, float maxX, float minY, float maxY)` (0x44DD10) on `ThePaths` (0x96F050)
    fn are_nodes_loaded_for_area(paths: GPtr<u8>, x0: u32, x1: u32, y0: u32, y1: u32) -> u8 = thiscall 0x44DD10 => unported;
}

/// `InteriorGroup_c::ArePathsLoaded()` (0x595380): the eight corners of the group entity's
/// collision box (c0 = min, c1 = (min.x, max.y, min.z), c2 = (max.x, max.y, min.z), c3 = (max.x,
/// min.y, min.z), c4..c7 the same at max.z), each transformed by the entity matrix (allocated and
/// rebuilt from the placement first when missing, checked per corner); their xy extents (start
/// +-999999, NaN never updates; the z extents are computed and unused) go to
/// `ThePaths.AreNodesLoadedForArea(lo.x, hi.x, lo.y, hi.y)`, whose result is returned.
#[hook(addr = 0x595380, conv = thiscall, name = "InteriorGroup_c::ArePathsLoaded", this = "InteriorGroup_c",
       expect = "81 EC 84 00 00 00",
       unverified = "world5-1", route = "L2", hazards = "none",
       note = "may allocate the entity matrix (matrix pool) when the entity has none")]
pub fn are_paths_loaded(this: GPtr<u8>) -> u8 {
    let ent = rd::<GPtr<u8>>(this, 8);
    let mi = i32::from(rd::<i16>(ent, 0x22));
    let col = rd::<GPtr<u8>>(g::<GPtr<u8>>((0xA9_B0C8 + mi * 4) as u32 as usize).read(), 0x14);
    let mn = [rd::<u32>(col, 0), rd::<u32>(col, 4), rd::<u32>(col, 8)];
    let mx = [rd::<u32>(col, 0xC), rd::<u32>(col, 0x10), rd::<u32>(col, 0x14)];
    let mut c: [[u32; 3]; 8] = [
        [mn[0], mn[1], mn[2]],
        [mn[0], mx[1], mn[2]],
        [mx[0], mx[1], mn[2]],
        [mx[0], mn[1], mn[2]],
        [mn[0], mn[1], mx[2]],
        [mn[0], mx[1], mx[2]],
        [mx[0], mx[1], mx[2]],
        [mx[0], mn[1], mx[2]],
    ];
    let big = f32::from_bits(0x4974_23F0);
    let mut lo = [big; 3];
    let mut hi = [-big; 3];
    for corner in c.iter_mut() {
        let e = rd::<GPtr<u8>>(this, 8);
        if rd::<u32>(e, 0x14) == 0 {
            allocate_matrix(e);
            simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let m = rd::<GPtr<u8>>(e, 0x14);
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(
            GPtr::from_raw(&mut out as *mut [u32; 3]).cast(),
            m,
            GPtr::from_raw(corner as *mut [u32; 3]).cast(),
        );
        *corner = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        for k in 0..3 {
            let v = f32::from_bits(corner[k]);
            if !x(v).fcom(x(lo[k])).test_parity(0x05) {
                lo[k] = v;
            }
        }
        for k in 0..3 {
            let v = f32::from_bits(corner[k]);
            if !x(v).fcom(x(hi[k])).test_any(0x41) {
                hi[k] = v;
            }
        }
    }
    are_nodes_loaded_for_area(
        GPtr::from_addr(0x96_F050),
        lo[0].to_bits(),
        hi[0].to_bits(),
        lo[1].to_bits(),
        hi[1].to_bits(),
    )
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ARE_PATHS_LOADED];
