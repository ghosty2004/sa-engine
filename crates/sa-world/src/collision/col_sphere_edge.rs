//! `CColSphere::IntersectEdge` / `IntersectRay` (batch `std8-world-1`): the hook entries of sa-math's models (L1
//! specs `CColSphereIntersectEdge`, `CColSphereIntersectRay`). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::col::{CColSphere, CColSurface, CSphere};
use sa_math::vector::CVector;

fn vec_at(p: GPtr<u8>) -> CVector {
    let f = |o: isize| p.byte_add(o).cast::<f32>().read();
    CVector::new(f(0), f(4), f(8))
}

fn put_vec(p: GPtr<u8>, v: CVector) {
    for (o, c) in [(0, v.x), (4, v.y), (8, v.z)] {
        p.byte_add(o).cast::<f32>().write(c);
    }
}

/// `CColSphere::IntersectEdge(const CVector& start, const CVector& end, CVector& p1, CVector& p2)` (0x4100E0,
/// thiscall `ret 0x10`, bool in AL): `sa_math::col::CColSphere::intersect_edge`; on a hit `p2` then `p1` are
/// written (the original also writes them in that order) and 1 is returned, else nothing is written and 0.
#[hook(addr = 0x4100E0, conv = thiscall, name = "CColSphere::IntersectEdge", expect = "83 EC 20 56 8B 74 24 28",
       unverified = "std8-world-1", route = "L1-pure", hazards = "x87-transcendental,ax-al-return",
       recs = "grove-walk", note = "FSQRT twice; AL result (upper EAX from fnstsw in the original)")]
pub fn col_sphere_intersect_edge(this: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>, p1: GPtr<u8>, p2: GPtr<u8>) -> u8 {
    let s = col_sphere_at(this);
    match s.intersect_edge(&vec_at(start), &vec_at(end)) {
        None => 0,
        Some((a, b)) => {
            put_vec(p2, b);
            put_vec(p1, a);
            1
        }
    }
}

fn col_sphere_at(this: GPtr<u8>) -> CColSphere {
    CColSphere {
        sphere: CSphere { center: vec_at(this), radius: this.byte_add(0xC).cast::<f32>().read() },
        surface: CColSurface {
            material: this.byte_add(0x10).read(),
            flags: this.byte_add(0x11).read(),
            lighting: this.byte_add(0x12).read(),
            light: this.byte_add(0x13).read(),
        },
    }
}

/// `CColSphere::IntersectRay(const CVector& origin, const CVector& dir, CVector& p1, CVector& p2)` (0x40FF20,
/// thiscall `ret 0x10`, bool in AL): `sa_math::col::CColSphere::intersect_ray`; on a hit `p1` then `p2` are written
/// and 1 is returned, else nothing is written and 0.
#[hook(addr = 0x40FF20, conv = thiscall, name = "CColSphere::IntersectRay", expect = "83 EC 18 D9 41 0C 56",
       unverified = "std8-world-1", route = "L1-pure", hazards = "x87-transcendental,ax-al-return",
       recs = "grove-walk", note = "FSQRT in CGeneral::SolveQuadratic; AL result")]
pub fn col_sphere_intersect_ray(this: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>, p1: GPtr<u8>, p2: GPtr<u8>) -> u8 {
    match col_sphere_at(this).intersect_ray(&vec_at(origin), &vec_at(dir)) {
        None => 0,
        Some((a, b)) => {
            put_vec(p1, a);
            put_vec(p2, b);
            1
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_COL_SPHERE_INTERSECT_EDGE, HOOK_COL_SPHERE_INTERSECT_RAY];
