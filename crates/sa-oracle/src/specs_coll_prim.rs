//! L1 specs for PORT-COLL `sa_world::collision::prim` (see `docs/arch/coll-port.md`): the pure
//! `CCollision` primitives. Every spec runs the hooked entry on the host on a copy of the scratch
//! image. Besides the generic float mix, [`small`] puts about half of the cases on a small grid
//! (halves in [-8, 8]) or a small uniform range, so overlaps, touching configurations and equal
//! coordinates (the strict/non-strict compare boundaries) are frequent.

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_world::collision::prim as p;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32], n: usize) -> u64 {
    let mut h: u64 = 0xC0_11;
    for x in &v[..n.min(v.len())] {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// About half of the cases: every float on a grid of halves in [-8, 8] (40 %) or uniform in ±10
/// (15 %); the rest keep the generic mix.
pub fn small(v: &mut [u32], lay: &Layout) {
    let n = lay.n_float;
    let mut r = Rng::new(seed(v, n));
    let mode = r.below(100);
    if mode >= 55 {
        return;
    }
    for x in v.iter_mut().take(n) {
        let f = if mode < 40 {
            (r.below(33) as i32 - 16) as f32 * 0.5
        } else {
            r.uniform(10.0)
        };
        *x = f.to_bits();
    }
}

/// `CColSphere`: centre, radius (floats), surface word (raw).
pub fn col_sphere() -> Region {
    Region {
        words: vec![W::F, W::F, W::F, W::F, W::R],
        kind: Kind::Any,
    }
}

/// `CColBox`: min, max (floats), surface word (raw).
pub fn col_box() -> Region {
    let mut w = vec![W::F; 6];
    w.push(W::R);
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// `CColLine`: start, pad, end, pad.
pub fn col_line() -> Region {
    floats(8)
}

fn spec2(
    key: &'static str,
    name: &'static str,
    addr: u32,
    ret: crate::specs::RetKind,
    regs: Vec<Region>,
) -> B {
    let args = (0..regs.len()).map(A::P).collect();
    b(key, name, addr, Cdecl, ret)
        .regions(regs)
        .args(args)
        .fix(|v, l| small(v, l))
}

pub fn all() -> Vec<Spec> {
    vec![
        spec2(
            "CCollisionTestSphereSphere",
            "CCollision::TestSphereSphere",
            0x411E70,
            Eax,
            vec![floats(4), floats(4)],
        )
        .preserve(sa_core::abi::PRESERVE_EDX)
        .rust(|c| ExpRet::Eax(p::test_sphere_sphere(tp(c, 0), tp(c, 1)))),
        spec2(
            "CCollisionTestSphereBox",
            "CCollision::TestSphereBox",
            0x4120C0,
            Al,
            vec![floats(4), floats(6)],
        )
        .rust(|c| ExpRet::Al(p::test_sphere_box(tp(c, 0), tp(c, 1)))),
        b(
            "CCollisionPointInTriangle",
            "CCollision::PointInTriangle",
            0x412700,
            Stdcall,
            Al,
        )
        .regions(vec![floats(3), floats(9)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| small(v, l))
        .rust(|c| ExpRet::Al(p::point_in_triangle(tp(c, 0), tp(c, 1)))),
        spec2(
            "CCollisionDistToLineSqr",
            "CCollision::DistToLineSqr",
            0x412850,
            St0,
            vec![floats(3), floats(3), floats(3)],
        )
        .rust(|c| st0(p::dist_to_line_sqr(tp(c, 0), tp(c, 1), tp(c, 2)))),
        spec2(
            "CCollisionDistToMathematicalLine",
            "CCollision::DistToMathematicalLine",
            0x412970,
            St0,
            vec![floats(3), floats(3), floats(3)],
        )
        .rust(|c| st0(p::dist_to_mathematical_line(tp(c, 0), tp(c, 1), tp(c, 2)))),
        b(
            "CCollisionDistToMathematicalLine2D",
            "CCollision::DistToMathematicalLine2D",
            0x412A30,
            Cdecl,
            St0,
        )
        .args(vec![A::F; 6])
        .fix(|v, l| small(v, l))
        .rust(|c| {
            st0(p::dist_to_mathematical_line_2d(
                c.f(0),
                c.f(1),
                c.f(2),
                c.f(3),
                c.f(4),
                c.f(5),
            ))
        }),
        b(
            "CCollisionDistAlongLine2D",
            "CCollision::DistAlongLine2D",
            0x412A80,
            Cdecl,
            St0,
        )
        .args(vec![A::F; 6])
        .fix(|v, l| small(v, l))
        .rust(|c| {
            st0(p::dist_along_line_2d(
                c.f(0),
                c.f(1),
                c.f(2),
                c.f(3),
                c.f(4),
                c.f(5),
            ))
        }),
        spec2(
            "CCollisionTestVerticalLineBox",
            "CCollision::TestVerticalLineBox",
            0x413080,
            Al,
            vec![floats(8), floats(6)],
        )
        .rust(|c| ExpRet::Al(p::test_vertical_line_box(tp(c, 0), tp(c, 1)))),
        b(
            "CCollisionTest2DLineAgainst2DLine",
            "CCollision::Test2DLineAgainst2DLine",
            0x4138D0,
            Cdecl,
            Al,
        )
        .args(vec![A::F; 8])
        .fix(|v, l| small(v, l))
        .rust(|c| {
            ExpRet::Al(p::test_2d_line_against_2d_line(
                c.f(0),
                c.f(1),
                c.f(2),
                c.f(3),
                c.f(4),
                c.f(5),
                c.f(6),
                c.f(7),
            ))
        }),
        b(
            "CCollisionGetBoundingBoxFromTwoSpheres",
            "CCollision::GetBoundingBoxFromTwoSpheres",
            0x415230,
            Cdecl,
            Void,
        )
        .regions(vec![raw(7), floats(4), floats(4)])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .fix(|v, l| small(v, l))
        .rust(|c| {
            p::get_bounding_box_from_two_spheres(tp(c, 0), tp(c, 1), tp(c, 2));
            ExpRet::Void
        }),
        spec2(
            "CCollisionGetPrincipleAxis",
            "CCollision::GetPrincipleAxis",
            0x4156D0,
            Eax,
            vec![floats(3)],
        )
        .rust(|c| ExpRet::Eax(p::get_principle_axis(tp(c, 0)))),
        spec2(
            "CCollisionTestLineBoxDW",
            "CCollision::TestLineBox_DW",
            0x412C70,
            Al,
            vec![floats(8), floats(6)],
        )
        .rust(|c| ExpRet::Al(p::test_line_box_dw(tp(c, 0), tp(c, 1)))),
        spec2(
            "CCollisionTestLineBox",
            "CCollision::TestLineBox",
            0x413070,
            Al,
            vec![floats(8), floats(6)],
        )
        .callees(&[0x412C70])
        .rust(|c| ExpRet::Al(p::test_line_box(tp(c, 0), tp(c, 1)))),
        spec2(
            "CCollisionTestLineSphere",
            "CCollision::TestLineSphere",
            0x417470,
            Al,
            vec![floats(8), floats(4)],
        )
        .rust(|c| ExpRet::Al(p::test_line_sphere(tp(c, 0), tp(c, 1)))),
        spec2(
            "CCollisionDistToLine",
            "CCollision::DistToLine",
            0x417610,
            St0,
            vec![floats(3), floats(3), floats(3)],
        )
        .rust(|c| st0(p::dist_to_line(tp(c, 0), tp(c, 1), tp(c, 2)))),
        b(
            "CCollisionClosestPointOnLine",
            "CCollision::ClosestPointOnLine",
            0x417FD0,
            Cdecl,
            Void,
        )
        .regions(vec![floats(3), floats(3), floats(3), raw(3)])
        .args(vec![A::P(0), A::P(1), A::P(2), A::P(3)])
        .fix(|v, l| small(v, l))
        .rust(|c| {
            p::closest_point_on_line(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3));
            ExpRet::Void
        }),
        spec2(
            "CCollisionCalculateColPointInsideBox",
            "CCollision::CalculateColPointInsideBox",
            0x411EC0,
            Void,
            vec![floats(6), floats(3), raw(11)],
        )
        .rust(|c| {
            p::calculate_col_point_inside_box(tp(c, 0), tp(c, 1), tp(c, 2));
            ExpRet::Void
        }),
        spec2(
            "CCollisionProcessSphereSphere",
            "CCollision::ProcessSphereSphere",
            0x416450,
            Al,
            vec![col_sphere(), col_sphere(), raw(11), floats(1)],
        )
        .callees(&[0x59C910])
        .rust(|c| ExpRet::Al(p::process_sphere_sphere(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3)))),
        spec2(
            "CCollisionProcessLineSphere",
            "CCollision::ProcessLineSphere",
            0x412AA0,
            Al,
            vec![col_line(), col_sphere(), raw(11), floats(1), raw(1)],
        )
        .callees(&[0x59C910])
        .patch(crossings_patch(0x412C53, 0x412C5C, 0xD0))
        .rust(|c| {
            let n = c.u(4, 0);
            let _g = crate::host_globals();
            p::PROCESS_LINE_NUM_CROSSINGS.set(n);
            let r = p::process_line_sphere(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3));
            let n = p::PROCESS_LINE_NUM_CROSSINGS.get();
            c.set_u(4, 0, n);
            ExpRet::Al(r)
        }),
        spec2(
            "CCollisionProcessSphereBox",
            "CCollision::ProcessSphereBox",
            0x412130,
            Al,
            vec![col_sphere(), col_box(), raw(11), floats(1)],
        )
        .callees(&[0x411EC0, 0x40FE60, 0x406DA0, 0x4119D0, 0x40FEC0])
        .rust(|c| ExpRet::Al(p::process_sphere_box(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3)))),
        spec2(
            "CCollisionSphereCastVsBBox",
            "CCollision::SphereCastVsBBox",
            0x415590,
            Al,
            vec![col_sphere(), col_sphere(), col_box()],
        )
        .callees(&[0x40EF50, 0x412C70])
        .rust(|c| ExpRet::Al(p::sphere_cast_vs_bbox(tp(c, 0), tp(c, 1), tp(c, 2)))),
        spec2(
            "CCollisionSphereCastVsSphere",
            "CCollision::SphereCastVsSphere",
            0x417F20,
            Al,
            vec![col_sphere(), col_sphere(), col_sphere()],
        )
        .callees(&[0x40EF50, 0x411E70, 0x417470])
        .rust(|c| ExpRet::Al(p::sphere_cast_vs_sphere(tp(c, 0), tp(c, 1), tp(c, 2)))),
        spec2(
            "CCollisionProcessDiscCollision",
            "CCollision::ProcessDiscCollision",
            0x413960,
            Al,
            vec![
                col_point(),
                rwmat(),
                col_disk(),
                col_point(),
                raw(1),
                floats(1),
                col_point(),
            ],
        )
        .callees(&[0x59C890, 0x59C790, 0x40FC80])
        .rust(|c| {
            ExpRet::Al(p::process_disc_collision(
                tp(c, 0),
                tp(c, 1),
                tp(c, 2),
                tp(c, 3),
                tp(c, 4),
                tp(c, 5),
                tp(c, 6),
            ))
        }),
        spec2(
            "CCollisionPointInPoly",
            "CCollision::PointInPoly",
            0x415730,
            Eax,
            vec![floats(3), raw(2), floats(3), floats(9)],
        )
        .callees(&[0x4156D0])
        .rust(|c| ExpRet::Eax(p::point_in_poly(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3)))),
        b(
            "CCollisionClosestSquaredDistanceBetweenFiniteLines",
            "CCollision::ClosestSquaredDistanceBetweenFiniteLines",
            0x415A40,
            Cdecl,
            St0,
        )
        .regions(vec![floats(3), floats(3), floats(3), floats(3)])
        .args(vec![A::P(0), A::P(1), A::P(2), A::P(3), A::F])
        .fix(|v, l| small(v, l))
        .rust(|c| {
            st0(p::closest_squared_distance_between_finite_lines(
                tp(c, 0),
                tp(c, 1),
                tp(c, 2),
                tp(c, 3),
                c.f(4),
            ))
        }),
        spec2(
            "CCollisionRayPolyPOP",
            "CCollision::RayPolyPOP",
            0x415620,
            Eax,
            vec![floats(3), floats(3), raw(2), floats(3), floats(3)],
        )
        .rust(|c| ExpRet::Eax(p::ray_poly_pop(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3), tp(c, 4)))),
        spec2(
            "CCollisionClosest3",
            "CCollision::Closest3",
            0x415950,
            Void,
            vec![floats(9), floats(3)],
        )
        .rust(|c| {
            p::closest3(tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
        spec2(
            "CCollisionClosestPointsOnPoly",
            "CCollision::ClosestPointsOnPoly",
            0x418100,
            Void,
            vec![raw(2), floats(9), floats(3), raw(9)],
        )
        .callees(&[0x417FD0])
        .rust(|c| {
            p::closest_points_on_poly(tp(c, 0), tp(c, 1), tp(c, 2), tp(c, 3));
            ExpRet::Void
        }),
        spec2(
            "CCollisionClosestPointOnPoly",
            "CCollision::ClosestPointOnPoly",
            0x418150,
            Void,
            vec![raw(2), floats(9), floats(3)],
        )
        .callees(&[0x417FD0, 0x415950])
        .rust(|c| {
            p::closest_point_on_poly(tp(c, 0), tp(c, 1), tp(c, 2));
            ExpRet::Void
        }),
    ]
}

/// `CColPoint`: point, pad, normal, pad (floats), surfaces (2 raw words), depth (float).
pub fn col_point() -> Region {
    let mut w = vec![W::F; 8];
    w.extend([W::R, W::R, W::F]);
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// `CColDisk` (0x24): centre, radius, surface word, axis, thickness.
pub fn col_disk() -> Region {
    let mut w = vec![W::F; 4];
    w.push(W::R);
    w.extend([W::F; 4]);
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// `ms_iProcessLineNumCrossings` (0x9655D0) is read and written through absolute operands (`mov eax,
/// [abs]` / `mov [abs], eax`): both are patched to the scratch word at byte offset `off` (the spec's
/// 1-word region; offsets follow `mspec`'s layout: regions start at 0x10, each padded by 0x10 and
/// rounded to 0x10).
pub fn crossings_patch(read_op: u32, write_op: u32, off: u32) -> Vec<(u32, Vec<u8>)> {
    let addr = crate::call::SCRATCH + off;
    vec![
        (read_op, addr.to_le_bytes().to_vec()),
        (write_op, addr.to_le_bytes().to_vec()),
    ]
}
