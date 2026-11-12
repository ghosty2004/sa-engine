//! `CPedGeometryAnalyser::ComputeRouteRoundSphere` (0x5F1890, batch `writerq-ai-1`): the detour point a ped takes
//! round a collision sphere. Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the bytes
//! are followed). Unverified (docs/verify/HOWTO.md).
#![allow(
    clippy::unreadable_literal,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CColSphere::IntersectPoint(const CVector&)` (0x410040, `ret 4`)
    fn sphere_intersect_point(s: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x410040 => unported;
    /// `CColSphere::IntersectRay(const CVector& start, const CVector& dir, CVector& p1, CVector& p2)` (0x40FF20, `ret 0x10`)
    fn sphere_intersect_ray(s: GPtr<u8>, start: GPtr<u8>, dir: GPtr<u8>, p1: GPtr<u8>, p2: GPtr<u8>) -> u8 = thiscall 0x40FF20 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

fn p3(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn put(p: GPtr<u8>, v: [f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        wr::<f32>(p, 4 * i as isize, *c);
    }
}

/// `CPedGeometryAnalyser::ComputeRouteRoundSphere(const CPed& ped, const CColSphere& sphere, const CVector& start,
/// const CVector& target, CVector& newTarget, CVector& detour)` (0x5F1890, cdecl). x87. P = the ped's position.
///
/// newTarget = target. When `sphere.IntersectPoint(target)` (the target, not the start as in gta-reversed): dir =
/// normalised f32(target - start), and a hit of the ray from P along dir sets newTarget = its second point. Then dir =
/// normalised f32(newTarget - P) and the ray from newTarget along dir: a miss → detour = newTarget, AL = 0. Else with A =
/// newTarget - P and B = p1 - P (registers, B.z stored as f32) and |B|² = (z² + y²) + x² not above |A|² (unordered
/// counts as not above): the ray from P along dir; a miss → AL = 1 with `detour` untouched; a hit → proj = (c - P)·dir
/// (z, y, x order, c the centre), q = (P.x + dir.x*proj, P.y + f32(dir.y*proj), f32(P.z + f32(dir.z*proj))) (x, y in
/// registers), u = normalised (f32(q.x - c.x), f32(q.y - c.y), f32(q.z - c.z)), detour = (f32(u.x*r + c.x),
/// f32(f32(u.y*r) + c.y), f32(f32(u.z*r) + c.z)), AL = 1. |B|² above |A|² → detour = newTarget, AL = 0.
///
/// EAX's upper bytes are the original's leftovers (the `detour` pointer or `IntersectRay`'s result); not reproduced.
#[hook(addr = 0x5F1890, conv = cdecl, name = "CPedGeometryAnalyser::ComputeRouteRoundSphere", expect = "8B 4C 24 04 8B 41 14",
       unverified = "writerq-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn compute_route_round_sphere(
    ped: GPtr<u8>,
    sphere: GPtr<u8>,
    start: GPtr<u8>,
    target: GPtr<u8>,
    new_target: GPtr<u8>,
    detour: GPtr<u8>,
) -> u8 {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if m.is_null() {
        ped.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    put(new_target, v3(target));
    let mut p1 = [0f32; 3];
    let mut p2 = [0f32; 3];
    if sphere_intersect_point(sphere, target) != 0 {
        let (t, s) = (v3(target), v3(start));
        let mut dir = [
            x(t[0]).fsub_m(s[0]).to_f32(),
            x(t[1]).fsub_m(s[1]).to_f32(),
            x(t[2]).fsub_m(s[2]).to_f32(),
        ];
        vector_normalise(p3(&mut dir));
        if sphere_intersect_ray(sphere, pp, p3(&mut dir), p3(&mut p1), p3(&mut p2)) != 0 {
            put(new_target, p2);
        }
    }
    let (nt, p) = (v3(new_target), v3(pp));
    let mut dir = [
        x(nt[0]).fsub_m(p[0]).to_f32(),
        x(nt[1]).fsub_m(p[1]).to_f32(),
        x(nt[2]).fsub_m(p[2]).to_f32(),
    ];
    vector_normalise(p3(&mut dir));
    if sphere_intersect_ray(sphere, new_target, p3(&mut dir), p3(&mut p1), p3(&mut p2)) == 0 {
        put(detour, v3(new_target));
        return 0;
    }
    let (nt, p) = (v3(new_target), v3(pp));
    let (ax, ay, az) = (
        x(nt[0]).fsub_m(p[0]),
        x(nt[1]).fsub_m(p[1]),
        x(nt[2]).fsub_m(p[2]),
    );
    let (bx, by) = (x(p1[0]).fsub_m(p[0]), x(p1[1]).fsub_m(p[1]));
    let bz = x(p1[2]).fsub_m(p[2]).to_f32();
    let la = az.fmul(az).fadd(ay.fmul(ay)).fadd(ax.fmul(ax));
    let lb = x(bz).fmul_m(bz).fadd(by.fmul(by)).fadd(bx.fmul(bx));
    if !lb.fcom(la).test_any(0x41) {
        put(detour, v3(new_target));
        return 0;
    }
    if sphere_intersect_ray(sphere, pp, p3(&mut dir), p3(&mut p1), p3(&mut p2)) == 0 {
        return 1;
    }
    let c = v3(sphere);
    let p = v3(pp);
    let proj = x(c[2])
        .fsub_m(p[2])
        .fmul_m(dir[2])
        .fadd(x(c[1]).fsub_m(p[1]).fmul_m(dir[1]))
        .fadd(x(c[0]).fsub_m(p[0]).fmul_m(dir[0]));
    let qy = x(dir[1]).fmul(proj).to_f32();
    let qz = x(dir[2]).fmul(proj).to_f32();
    let qx = x(dir[0]).fmul(proj).fadd_m(p[0]);
    let qy = x(qy).fadd_m(p[1]);
    let qz = x(qz).fadd_m(p[2]).to_f32();
    let mut u = [
        qx.fsub_m(c[0]).to_f32(),
        qy.fsub_m(c[1]).to_f32(),
        x(qz).fsub_m(c[2]).to_f32(),
    ];
    vector_normalise(p3(&mut u));
    let r = x(rd::<f32>(sphere, 0xC));
    let dy = x(u[1]).fmul(r).to_f32();
    let dz = x(u[2]).fmul(r).to_f32();
    put(
        detour,
        [
            x(u[0]).fmul(r).fadd_m(c[0]).to_f32(),
            x(dy).fadd_m(c[1]).to_f32(),
            x(dz).fadd_m(c[2]).to_f32(),
        ],
    );
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_COMPUTE_ROUTE_ROUND_SPHERE];
