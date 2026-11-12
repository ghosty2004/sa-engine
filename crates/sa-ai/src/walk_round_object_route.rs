//! `CTaskComplexWalkRoundObject::ComputeRoute(CPed*)` (0x6551D0, batch `std12-ai-1`): the route round an object a
//! walking ped bumps into (Grove Street's ambient peds). Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(CPed*, CEntity*, CVector& target, CPointRoute*,
    /// int32)` (0x5F6110)
    fn route_round_entity_bbox(ped: GPtr<u8>, e: GPtr<u8>, target: GPtr<u8>, route: GPtr<u8>, flag: u32) -> u32 = cdecl 0x5F6110 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes(float z, CEntity*, CVector (*normals)[4], float* d)`
    /// (0x5F3660), z as bits
    fn bbox_planes(z: u32, e: GPtr<u8>, normals: GPtr<u8>, d: GPtr<u8>) = cdecl 0x5F3660 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, CEntity*)` (0x5F3AC0)
    fn hit_side(p: GPtr<u8>, e: GPtr<u8>) -> u32 = cdecl 0x5F3AC0 => unported;
}

/// The bounding-box route expansion global (`[0x8D22B0]`, set to 0.7 for this one call).
const ROUTE_EXPAND: usize = 0x8D22B0;

fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskComplexWalkRoundObject::ComputeRoute(CPed* ped)` (0x6551D0, thiscall, `ret 4`, ST0). With [0x8D22B0]
/// temporarily 0.7: `ComputeRouteRoundEntityBoundingBox(ped, object (+0x1C), &target (+0x10), route (+0x20), 0)`.
/// Then the object's bounding-box planes at the ped's z and the ped's hit side `s`; returns the signed distance
/// `((n.z * p.z + n.y * p.y) + n.x * p.x) + d` of the ped position to plane `s` (x87).
#[hook(addr = 0x6551D0, conv = thiscall, name = "(missed by ghidra: cat7-call)", expect = "83 EC 44 A1 B0 22 8D 00",
       unverified = "std12-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn walk_round_object_compute_route(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let saved = g::<u32>(ROUTE_EXPAND).read();
    g::<u32>(ROUTE_EXPAND).write(0x3F33_3333);
    route_round_entity_bbox(ped, rd::<GPtr<u8>>(this, 0x1C), this.byte_add(0x10), rd::<GPtr<u8>>(this, 0x20), 0);
    g::<u32>(ROUTE_EXPAND).write(saved);
    let mut normals = [0f32; 12];
    let mut d = [0f32; 4];
    bbox_planes(
        rd::<u32>(pos_of(ped), 8),
        rd::<GPtr<u8>>(this, 0x1C),
        GPtr::from_raw(normals.as_mut_ptr()).cast(),
        GPtr::from_raw(d.as_mut_ptr()).cast(),
    );
    // the side is 0..=3; `& 3` keeps an out-of-range value inside the arrays (the original would read its stack)
    let s = hit_side(pos_of(ped), rd::<GPtr<u8>>(this, 0x1C)) as usize & 3;
    let p = pos_of(ped);
    let n = &normals[3 * s..3 * s + 3];
    x(n[2]).fmul_m(rd::<f32>(p, 8)).fadd(x(n[1]).fmul_m(rd::<f32>(p, 4))).fadd(x(n[0]).fmul_m(rd::<f32>(p, 0))).fadd_m(d[s])
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_WALK_ROUND_OBJECT_COMPUTE_ROUTE];
