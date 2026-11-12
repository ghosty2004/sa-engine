//! STD-15 (batch `std15-ai-2`): hand ports of the three `CPedGeometryAnalyser` bodies AI-GEN got wrong (it called
//! `ComputeEntityBoundingBoxCornersUncached` 0x5F1FA0 as a one-argument thiscall; the original pushes three cdecl
//! arguments, docs/arch/platform.md 21.2): `ComputeEntityBoundingBoxPlanesUncachedAll` (0x5F2B80),
//! `ComputeMoveDirToAvoidEntity` (0x5F3730) and `ComputeEntityBoundingBoxCentreUncachedAll` (0x5F3B40). Ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector* corners)` (0x5F1FA0, cdecl)
    fn bbox_corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) -> u8 = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncached(float z, const CVector* corners, CVector* normals, float* ds)` (0x5F1670, cdecl)
    fn bbox_planes_uncached(z: u32, corners: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F1670 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncached(float z, const CVector* corners, CVector& out)` (0x5F1600, cdecl)
    fn bbox_centre_uncached(z: u32, corners: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F1600 => unported;
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncachedAll(float z, CEntity&, CVector* normals, float* ds)`
/// (0x5F2B80, cdecl): the four corners on the stack (0x30 bytes), then the planes through them.
#[hook(addr = 0x5F2B80, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncachedAll",
       expect = "8B 4C 24 08 83 EC 30", unverified = "std15-ai-2", route = "L2", hazards = "none", recs = "car-enter")]
pub fn bbox_planes_uncached_all(z: u32, e: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) {
    let mut corners = [0u32; 12];
    let cp: GPtr<u8> = GPtr::from_raw(corners.as_mut_ptr()).cast();
    bbox_corners_uncached(z, e, cp);
    bbox_planes_uncached(z, cp, normals, ds);
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncachedAll(float z, CEntity&, CVector& out)` (0x5F3B40,
/// cdecl): the four corners on the stack, then their centre.
#[hook(addr = 0x5F3B40, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncachedAll",
       expect = "8B 4C 24 08 83 EC 30", unverified = "std15-ai-2", route = "L2", hazards = "none", recs = "car-enter")]
pub fn bbox_centre_uncached_all(z: u32, e: GPtr<u8>, out: GPtr<u8>) {
    let mut corners = [0u32; 12];
    let cp: GPtr<u8> = GPtr::from_raw(corners.as_mut_ptr()).cast();
    bbox_corners_uncached(z, e, cp);
    bbox_centre_uncached(z, cp, out);
}

/// `CPedGeometryAnalyser::ComputeMoveDirToAvoidEntity(const CPed&, CEntity&, CVector& out)` (0x5F3730, cdecl). The
/// entity's bounding-box planes at the ped's z (corners, then planes, both uncached); a = plane 1 at the ped's
/// position (`(n.y p.y + n.z p.z) + n.x p.x + d`, unrounded), b = plane 3 (rounded to f32). out = normal 1 when
/// a > 0, else normal 3 when b > 0, else normal 3 when a <= b and normal 1 otherwise (gta-reversed has the last
/// choice the other way round; the bytes pick normal 3 for a <= b).
#[hook(addr = 0x5F3730, conv = cdecl, name = "CPedGeometryAnalyser::ComputeMoveDirToAvoidEntity-OG",
       expect = "83 EC 74 56 8B 74 24 7C", unverified = "std15-ai-2", route = "L2", hazards = "none", recs = "car-enter",
       note = "a <= b picks normal 3 (gta-reversed: normal 1)")]
pub fn compute_move_dir_to_avoid_entity(ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) {
    let pos = || {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            ped.byte_add(4)
        } else {
            m.byte_add(0x30)
        }
    };
    let z = rd::<u32>(pos(), 8);
    let mut corners = [0u32; 12];
    let cp: GPtr<u8> = GPtr::from_raw(corners.as_mut_ptr()).cast();
    let mut normals = [0f32; 12];
    let np: GPtr<u8> = GPtr::from_raw(normals.as_mut_ptr()).cast();
    let mut ds = [0f32; 4];
    let dp: GPtr<u8> = GPtr::from_raw(ds.as_mut_ptr()).cast();
    bbox_corners_uncached(z, e, cp);
    bbox_planes_uncached(z, cp, np, dp);
    let plane = |i: isize| {
        let p = pos();
        let n = np.byte_add(i * 12);
        x(rd::<f32>(n, 4))
            .fmul_m(rd::<f32>(p, 4))
            .fadd(x(rd::<f32>(n, 8)).fmul_m(rd::<f32>(p, 8)))
            .fadd(x(rd::<f32>(n, 0)).fmul_m(rd::<f32>(p, 0)))
            .fadd_m(rd::<f32>(dp, i * 4))
    };
    let a = plane(1);
    let b = plane(3).to_f32();
    let copy = |i: isize| {
        let n = np.byte_add(i * 12);
        for k in 0..3isize {
            wr::<u32>(out, k * 4, rd::<u32>(n, k * 4));
        }
    };
    let zero = x(g::<f32>(0x858B50).read());
    if !a.fcom(zero).test_any(0x41) {
        copy(1);
    } else if !x(b).fcom(zero).test_any(0x41) || a.fcom(x(b)).test_any(0x41) {
        copy(3);
    } else {
        copy(1);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_BBOX_PLANES_UNCACHED_ALL,
    HOOK_BBOX_CENTRE_UNCACHED_ALL,
    HOOK_COMPUTE_MOVE_DIR_TO_AVOID_ENTITY,
];
