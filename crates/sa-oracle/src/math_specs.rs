//! L1 specs of the MATH subsystem (MATH-2), built with [`crate::mspec`]. One spec per catalogue row
//! that can run without state beyond its arguments; see `docs/re/math-port.md` for the rows that need
//! L2 and why.

use crate::call::{Conv::*, SCRATCH};
use crate::mspec::*;
use crate::specs::{ExpRet, FPATAN_REASON, RetKind::*, Route, Spec};
use sa_math::col::{CBox, CColBox, CColLine, CColSphere, CSphere};
use sa_math::compressed::CCompressedMatrixNotAligned;
use sa_math::general as g;
use sa_math::maths;
use sa_math::matrix::{self as mx, CMatrix};
use sa_math::quaternion::CQuaternion;
use sa_math::rect::CRect;
use sa_math::vector::{self as vc, CVector, CVector2D};
use sa_math::x87::X87;

pub const FSIN_REASON: &str = "FSIN/FCOS: Unicorn (QEMU 5.0.1) computes them with the host's double sin/cos \
     (no 66-bit-pi argument reduction, 53-bit results; measured, docs/re/l1-harness.md)";

const FSIN: Route = Route::L2 { reason: FSIN_REASON };
const FPATAN: Route = Route::L2 {
    reason: FPATAN_REASON,
};

/// `_ftol2`, `rand`, `floor`, `RwMatrixUpdate`.
const FTOL: u32 = 0x821B40;
const RAND: u32 = 0x821B1E;
const RW_MATRIX_UPDATE: u32 = 0x7F18A0;
const FLOOR: u32 = 0x8219F0;
const SSE2_AVAILABLE: u32 = 0xC9C400;
const SIN_TABLE: u32 = 0xBB3E00;

fn rect(c: &Ctx, r: usize) -> CRect {
    CRect {
        left: c.fl(r, 0),
        top: c.fl(r, 1),
        right: c.fl(r, 2),
        bottom: c.fl(r, 3),
    }
}
fn set_rect(c: &mut Ctx, r: usize, x: &CRect) {
    c.set_fl(r, 0, x.left);
    c.set_fl(r, 1, x.top);
    c.set_fl(r, 2, x.right);
    c.set_fl(r, 3, x.bottom);
}
fn rd_quat(c: &Ctx, r: usize) -> CQuaternion {
    CQuaternion {
        x: c.fl(r, 0),
        y: c.fl(r, 1),
        z: c.fl(r, 2),
        w: c.fl(r, 3),
    }
}
fn colsphere(c: &Ctx, r: usize) -> CColSphere {
    let mut s = CColSphere::default();
    s.sphere.center = c.vec(r);
    s.sphere.radius = c.fl(r, 3);
    s
}
fn bx(c: &Ctx, r: usize) -> CBox {
    CBox {
        min: c.vec(r),
        max: CVector::new(c.fl(r, 3), c.fl(r, 4), c.fl(r, 5)),
    }
}
fn set_box(c: &mut Ctx, r: usize, b: &CBox) {
    c.set_vec(r, b.min);
    c.set_fl(r, 3, b.max.x);
    c.set_fl(r, 4, b.max.y);
    c.set_fl(r, 5, b.max.z);
}
fn eax(c: &Ctx, r: usize) -> ExpRet {
    ExpRet::Eax(c.addr(r))
}
fn boolean(b: bool) -> ExpRet {
    ExpRet::Eax(b as u32)
}
fn al(b: bool) -> ExpRet {
    ExpRet::Al(b as u8)
}

/// `mov eax, [SCRATCH + 0x10]; ret` at `rand`: the first region (one raw word at scratch+0x10) is the
/// value `rand()` returns.
fn sin_table() -> &'static [f32; 256] {
    static T: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    T.get_or_init(maths::init_maths_tables)
}

/// The `.bss` sine table (0xBB3E00) as `CMaths::InitMathsTables` leaves it (its values are checked
/// against the CPU by the `fsin_fcos_model_matches_the_cpu` test).
fn sin_table_patch() -> Vec<(u32, Vec<u8>)> {
    let t: Vec<u8> = sin_table().iter().flat_map(|v| v.to_le_bytes()).collect();
    vec![(SIN_TABLE, t)]
}

fn rand_stub() -> Vec<(u32, Vec<u8>)> {
    let mut p = vec![0xA1];
    p.extend_from_slice(&(SCRATCH + 0x10).to_le_bytes());
    p.push(0xC3);
    vec![(RAND, p)]
}

/// A matrix setter: `this` only, plus `n` float arguments.
fn mat_setter(
    key: &'static str,
    name: &'static str,
    addr: u32,
    n: usize,
    route: Route,
    f: fn(&mut CMatrix, &[f32]),
) -> Spec {
    let mut args = vec![A::P(0)];
    args.extend(std::iter::repeat_n(A::F, n));
    b(key, name, addr, Thiscall, Void)
        .regions(vec![mat()])
        .args(args)
        .route(route)
        .rust(move |c| {
            let mut m = c.mat(0);
            let fs: Vec<f32> = (1..=n).map(|k| c.f(k)).collect();
            f(&mut m, &fs);
            c.set_mat(0, &m);
            ExpRet::Void
        })
}

pub fn all() -> Vec<Spec> {
    let mut v = vec![
        // ------------------------------------------------------------------ CRect
        b("CRectIsFlipped", "CRect::IsFlipped", 0x404190, Thiscall, Eax)
            .regions(vec![floats(4)])
            .args(vec![A::P(0)])
            .rust(|c| boolean(rect(c, 0).is_flipped())),
        b("CRectRestrict", "CRect::Restrict", 0x404200, Thiscall, Eax)
            .regions(vec![floats(4), floats(4)])
            .args(vec![A::P(0), A::P(1)])
            .alias(1, 0, 10)
            .rust(|c| {
                let (mut r, o) = (rect(c, 0), rect(c, c.ar(1)));
                r.restrict(&o);
                set_rect(c, 0, &r);
                eax(c, 0)
            }),
        b("CRectResize", "CRect::Resize", 0x404260, Thiscall, Void)
            .regions(vec![floats(4)])
            .args(vec![A::P(0), A::F, A::F])
            .rust(|c| {
                let mut r = rect(c, 0);
                r.resize(c.f(1), c.f(2));
                set_rect(c, 0, &r);
                ExpRet::Void
            }),
        b(
            "CRectIsPointInside",
            "CRect::IsPointInside",
            0x404290,
            Thiscall,
            Al,
        )
        .regions(vec![floats(4), vec2()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| al(rect(c, 0).is_point_inside(&c.v2(1)))),
        b(
            "CRectIsPointInsideTolerance",
            "CRect::IsPointInside(tolerance)",
            0x4042D0,
            Thiscall,
            Al,
        )
        .regions(vec![floats(4), vec2()])
        .args(vec![A::P(0), A::P(1), A::F])
        .rust(|c| al(rect(c, 0).is_point_inside_tolerance(&c.v2(1), c.f(2)))),
        b(
            "CRectSetFromCenter",
            "CRect::SetFromCenter",
            0x43E020,
            Thiscall,
            Eax,
        )
        .regions(vec![floats(4)])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .rust(|c| {
            let mut r = rect(c, 0);
            r.set_from_center(c.f(1), c.f(2), c.f(3));
            set_rect(c, 0, &r);
            eax(c, 0)
        }),
        b("CRectGetCenter", "CRect::GetCenter", 0x43E050, Thiscall, Void)
            .regions(vec![floats(4), out1(), out1()])
            .args(vec![A::P(0), A::P(1), A::P(2)])
            .rust(|c| {
                let (x, y) = rect(c, 0).get_center();
                c.set_fl(1, 0, x);
                c.set_fl(2, 0, y);
                ExpRet::Void
            }),
        b(
            "CRectStretchToPoint",
            "CRect::StretchToPoint",
            0x5327F0,
            Thiscall,
            Eax,
        )
        .regions(vec![floats(4)])
        .args(vec![A::P(0), A::F, A::F])
        .rust(|c| {
            let mut r = rect(c, 0);
            r.stretch_to_point(c.f(1), c.f(2));
            set_rect(c, 0, &r);
            eax(c, 0)
        }),
        // ------------------------------------------------------------------ CVector / CVector2D
        b(
            "CVectorMagnitude2D",
            "CVector::Magnitude2D",
            0x406D50,
            Thiscall,
            St0,
        )
        .regions(vec![vec3()])
        .args(vec![A::P(0)])
        .rust(|c| st0(c.vec(0).magnitude_2d_x87())),
        b("CVectorMagnitude", "CVector::Magnitude", 0x4082C0, Thiscall, St0)
            .regions(vec![vec3()])
            .args(vec![A::P(0)])
            .rust(|c| st0(c.vec(0).magnitude_x87())),
        b(
            "CVectorNormaliseAndMag",
            "CVector::NormaliseAndMag",
            0x59C970,
            Thiscall,
            St0,
        )
        .regions(vec![vec3()])
        .args(vec![A::P(0)])
        .rust(|c| {
            let mut v = c.vec(0);
            let r = v.normalise_and_mag_x87();
            c.set_vec(0, v);
            st0(r)
        }),
        b(
            "CVectorDotProduct",
            "CVector::DotProduct(v3d*, v3d*)",
            0x40FDB0,
            Cdecl,
            St0,
        )
        .regions(vec![vec3(), vec3()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| st0(vc::dot_product_x87(&c.vec(0), &c.vec(1)))),
        b("CVectorSum", "CVector::Sum", 0x40FDD0, Thiscall, Void)
            .regions(vec![vec3(), vec3(), vec3()])
            .args(vec![A::P(0), A::P(1), A::P(2)])
            .alias(1, 0, 25)
            .rust(|c| {
                let r = CVector::sum(&c.vec(c.ar(1)), &c.vec(2));
                c.set_vec(0, r);
                ExpRet::Void
            }),
        b(
            "CVectorDifference",
            "CVector::Difference",
            0x40FE00,
            Thiscall,
            Void,
        )
        .regions(vec![vec3(), vec3(), vec3()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .alias(2, 0, 25)
        .rust(|c| {
            let r = CVector::difference(&c.vec(1), &c.vec(c.ar(2)));
            c.set_vec(0, r);
            ExpRet::Void
        }),
        b(
            "CVectorCrossProduct",
            "CrossProduct(CVector*, const CVector*, const CVector*)",
            0x59C730,
            Cdecl,
            Eax,
        )
        .regions(vec![vec3(), vec3(), vec3()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .alias(1, 0, 20)
        .rust(|c| {
            let r = vc::cross_product(&c.vec(c.ar(1)), &c.vec(2));
            c.set_vec(0, r);
            eax(c, 0)
        }),
        b("CVectorCrossOG", "CVector::Cross_OG", 0x70F890, Thiscall, Void)
            .regions(vec![vec3(), vec3(), vec3()])
            .args(vec![A::P(0), A::P(1), A::P(2)])
            .alias(1, 0, 30)
            .rust(|c| {
                let mut out = c.vec(0);
                if c.aliased {
                    let a = out;
                    CVector::cross_og_aliased(&mut out, &a, &c.vec(2), true, false);
                } else {
                    out = CVector::cross_og(&c.vec(1), &c.vec(2));
                }
                c.set_vec(0, out);
                ExpRet::Void
            }),
        b(
            "CVectorFromMultiply",
            "CVector::FromMultiply",
            0x59C670,
            Thiscall,
            Void,
        )
        .regions(vec![vec3(), mat(), vec3()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .alias(2, 0, 30)
        .rust(|c| {
            let m = c.mat(1);
            let mut out = c.vec(0);
            if c.aliased {
                out.from_multiply_in_place(&m);
            } else {
                out = CVector::from_multiply(&m, &c.vec(2));
            }
            c.set_vec(0, out);
            ExpRet::Void
        }),
        b(
            "CVectorFromMultiply3x3",
            "CVector::FromMultiply3x3",
            0x59C6D0,
            Thiscall,
            Void,
        )
        .regions(vec![vec3(), mat(), vec3()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .alias(2, 0, 30)
        .rust(|c| {
            let m = c.mat(1);
            let mut out = c.vec(0);
            if c.aliased {
                out.from_multiply_3x3_in_place(&m);
            } else {
                out = CVector::from_multiply_3x3(&m, &c.vec(2));
            }
            c.set_vec(0, out);
            ExpRet::Void
        }),
        b(
            "CMatrixMulVec",
            "operator*(const CMatrix&, const CVector&)",
            0x59C890,
            Cdecl,
            Eax,
        )
        .regions(vec![vec3(), mat(), vec3()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .alias(2, 0, 20)
        .rust(|c| {
            let r = vc::matrix_mul_vector(&c.mat(1), &c.vec(c.ar(2)));
            c.set_vec(0, r);
            eax(c, 0)
        }),
        b(
            "CVector2DMagnitude",
            "CVector2D::Magnitude",
            0x420860,
            Thiscall,
            St0,
        )
        .regions(vec![vec2()])
        .args(vec![A::P(0)])
        .rust(|c| st0(c.v2(0).magnitude_x87())),
        b("CVector2DAssign", "CVector2D::operator=", 0x43E110, Thiscall, Eax)
            .regions(vec![vec2(), vec2()])
            .args(vec![A::P(0), A::P(1)])
            .rust(|c| {
                let mut v = c.v2(0);
                v.assign(&c.v2(1));
                c.set_v2(0, v);
                eax(c, 0)
            }),
        b(
            "CVector2DNormalise",
            "CVector2D::Normalise",
            0x44E480,
            Thiscall,
            Void,
        )
        .regions(vec![vec2()])
        .args(vec![A::P(0)])
        .rust(|c| {
            let mut v: CVector2D = c.v2(0);
            v.normalise();
            c.set_v2(0, v);
            ExpRet::Void
        }),
        // ------------------------------------------------------------------ collision primitives
        b("CBoxSet", "CBox::Set", 0x40EDE0, Thiscall, Void)
            .regions(vec![floats(6), vec3(), vec3()])
            .args(vec![A::P(0), A::P(1), A::P(2)])
            .rust(|c| {
                let mut x = bx(c, 0);
                x.set(&c.vec(1), &c.vec(2));
                set_box(c, 0, &x);
                ExpRet::Void
            }),
        b("CBoxRecalc", "CBox::Recalc", 0x40EE20, Thiscall, Void)
            .regions(vec![floats(6)])
            .args(vec![A::P(0)])
            .rust(|c| {
                let mut x = bx(c, 0);
                x.recalc();
                set_box(c, 0, &x);
                ExpRet::Void
            }),
        b("CColBoxSet", "CColBox::Set", 0x40EE70, Thiscall, Void)
            .regions(vec![
                Region {
                    words: vec![W::F, W::F, W::F, W::F, W::F, W::F, W::R],
                    kind: Kind::Any,
                },
                vec3(),
                vec3(),
            ])
            .args(vec![
                A::P(0),
                A::P(1),
                A::P(2),
                A::I(g_byte),
                A::I(g_byte),
                A::I(g_byte),
            ])
            .rust(|c| {
                let mut x = CColBox {
                    bx: bx(c, 0),
                    ..Default::default()
                };
                let (m, f, l) = (c.i(3) as u8, c.i(4) as u8, c.i(5) as u8);
                x.set(&c.vec(1), &c.vec(2), m, f, l);
                set_box(c, 0, &x.bx);
                c.set_byte(0, 0x18, m);
                c.set_byte(0, 0x19, f);
                c.set_byte(0, 0x1A, l);
                ExpRet::Void
            }),
        b("CColLineSet", "CColLine::Set", 0x40EF10, Thiscall, Void)
            .regions(vec![
                Region {
                    words: vec![W::F, W::F, W::F, W::R, W::F, W::F, W::F, W::R],
                    kind: Kind::Any,
                },
                vec3(),
                vec3(),
            ])
            .args(vec![A::P(0), A::P(1), A::P(2)])
            .rust(|c| {
                let mut l = CColLine::default();
                l.set(&c.vec(1), &c.vec(2));
                c.set_vec(0, l.start);
                c.set_fl(0, 4, l.end.x);
                c.set_fl(0, 5, l.end.y);
                c.set_fl(0, 6, l.end.z);
                ExpRet::Void
            }),
        b("CSphereSet", "CSphere::Set", 0x40FCF0, Thiscall, Void)
            .regions(vec![floats(4), vec3()])
            .args(vec![A::P(0), A::F, A::P(1)])
            .rust(|c| {
                let mut s = CSphere::default();
                s.set(c.f(1), &c.vec(1));
                c.set_vec(0, s.center);
                c.set_fl(0, 3, s.radius);
                ExpRet::Void
            }),
        b("CColSphereSet", "CColSphere::Set", 0x40FD10, Thiscall, Void)
            .regions(vec![
                Region {
                    words: vec![W::F, W::F, W::F, W::F, W::R],
                    kind: Kind::Any,
                },
                vec3(),
            ])
            .args(vec![
                A::P(0),
                A::F,
                A::P(1),
                A::I(g_byte),
                A::I(g_byte),
                A::I(g_byte),
            ])
            .rust(|c| {
                let mut s = CColSphere::default();
                let (m, f, l) = (c.i(3) as u8, c.i(4) as u8, c.i(5) as u8);
                s.set(c.f(1), &c.vec(1), m, f, l);
                c.set_vec(0, s.sphere.center);
                c.set_fl(0, 3, s.sphere.radius);
                c.set_byte(0, 0x10, m);
                c.set_byte(0, 0x11, f);
                c.set_byte(0, 0x12, l);
                ExpRet::Void
            }),
        b(
            "CColSphereIntersectPoint",
            "CColSphere::IntersectPoint",
            0x410040,
            Thiscall,
            Eax,
        )
        .regions(vec![floats(5), vec3()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| boolean(colsphere(c, 0).intersect_point(&c.vec(1)))),
        b(
            "CColSphereIntersectSphere",
            "CColSphere::IntersectSphere",
            0x410090,
            Thiscall,
            Al,
        )
        .regions(vec![floats(5), floats(5)])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| al(colsphere(c, 0).intersect_sphere(&colsphere(c, 1)))),
        b(
            "CColSphereIntersectRay",
            "CColSphere::IntersectRay",
            0x40FF20,
            Thiscall,
            Al,
        )
        .regions(vec![floats(5), vec3(), vec3(), out_vec(), out_vec()])
        .args(vec![A::P(0), A::P(1), A::P(2), A::P(3), A::P(4)])
        .callees(&[0x53CE30])
        .rust(|c| match colsphere(c, 0).intersect_ray(&c.vec(1), &c.vec(2)) {
            None => al(false),
            Some((p1, p2)) => {
                c.set_vec(3, p1);
                c.set_vec(4, p2);
                al(true)
            }
        }),
        b(
            "CColSphereIntersectEdge",
            "CColSphere::IntersectEdge",
            0x4100E0,
            Thiscall,
            Al,
        )
        .regions(vec![floats(5), vec3(), vec3(), out_vec(), out_vec()])
        .args(vec![A::P(0), A::P(1), A::P(2), A::P(3), A::P(4)])
        .callees(&[0x59C910])
        .rust(|c| match colsphere(c, 0).intersect_edge(&c.vec(1), &c.vec(2)) {
            None => al(false),
            Some((p1, p2)) => {
                c.set_vec(4, p2);
                c.set_vec(3, p1);
                al(true)
            }
        }),
        // ------------------------------------------------------------------ CQuaternion
        b(
            "CQuaternionNormalise",
            "CQuaternion::Normalise",
            0x4D1610,
            Thiscall,
            Void,
        )
        .regions(vec![quat()])
        .args(vec![A::P(0)])
        .rust(|c| {
            let mut q = rd_quat(c, 0);
            q.normalise();
            for (k, x) in [q.x, q.y, q.z, q.w].into_iter().enumerate() {
                c.set_fl(0, k, x);
            }
            ExpRet::Void
        }),
        b(
            "CQuaternionGet",
            "CQuaternion::Get(RwMatrix*)",
            0x59C080,
            Thiscall,
            Void,
        )
        .regions(vec![quat(), rwmat()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| {
            let mut m = c.rw(1);
            rd_quat(c, 0).get(&mut m);
            c.set_rw(1, &m);
            ExpRet::Void
        }),
        b(
            "CMatrixSetRotateQuat",
            "CMatrix::SetRotate(const CQuaternion&)",
            0x59BBF0,
            Thiscall,
            Void,
        )
        .regions(vec![mat(), quat()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| {
            let mut m = c.mat(0);
            m.set_rotate_quat(&rd_quat(c, 1));
            c.set_mat(0, &m);
            ExpRet::Void
        }),
        // ------------------------------------------------------------------ CGeneral
        b(
            "GetRandomNumberInRangeInt",
            "CGeneral::GetRandomNumberInRange(int, int)",
            0x407180,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(1)])
        .args(vec![A::I(g_small_int), A::I(g_small_int)])
        .callees(&[RAND, FTOL])
        .patch(rand_stub())
        .rust(|c| {
            ExpRet::Eax(
                g::get_random_number_in_range_i32(c.u(0, 0) as i32, c.i(0) as i32, c.i(1) as i32) as u32,
            )
        }),
        b(
            "GetRandomNumberInRangeFloat",
            "CGeneral::GetRandomNumberInRange(float, float)",
            0x41BD90,
            Cdecl,
            St0,
        )
        .regions(vec![raw(1)])
        .args(vec![A::F, A::F])
        .callees(&[RAND])
        .patch(rand_stub())
        .rust(|c| {
            st0(g::get_random_number_in_range_f32_x87(
                c.u(0, 0) as i32,
                c.f(0),
                c.f(1),
            ))
        }),
        b(
            "GetNodeHeadingFromVector",
            "CGeneral::GetNodeHeadingFromVector",
            0x53CDC0,
            Cdecl,
            Eax,
        )
        .args(vec![A::F, A::F])
        .callees(&[0x53CBE0, FLOOR, FTOL])
        .route(FPATAN)
        // the CRT's `__sse2_available` (0xC9C400) as the game has it at runtime: floor takes its SSE2 path
        .patch(vec![(SSE2_AVAILABLE, 1u32.to_le_bytes().to_vec())])
        .reads(vec![(SSE2_AVAILABLE, SSE2_AVAILABLE + 4)])
        // a NaN angle makes floor call the CRT's error handler (errno: global state): not generated
        .fix(|v, _| {
            let (x, y) = (f32::from_bits(v[0]), f32::from_bits(v[1]));
            if g::get_radian_angle_between_points_x87(x, y, 0.0, 0.0).is_nan() {
                v[0] = 1.0f32.to_bits();
                v[1] = (-2.0f32).to_bits();
            }
        })
        .rust(|c| ExpRet::Eax(g::get_node_heading_from_vector(c.f(0), c.f(1)) as u32)),
        b("CMathsGetSinFast", "CMaths::GetSinFast", 0x4A1340, Cdecl, St0)
            .args(vec![A::F])
            .callees(&[FTOL])
            .patch(sin_table_patch())
            .reads(vec![(SIN_TABLE, SIN_TABLE + 0x400)])
            .rust(|c| st0(X87::from_f32(maths::get_sin_fast(sin_table(), c.f(0))))),
        b("CMathsGetCosFast", "CMaths::GetCosFast", 0x4A1360, Cdecl, St0)
            .args(vec![A::F])
            .callees(&[FTOL])
            .patch(sin_table_patch())
            .reads(vec![(SIN_TABLE, SIN_TABLE + 0x400)])
            .rust(|c| st0(X87::from_f32(maths::get_cos_fast(sin_table(), c.f(0))))),
        b(
            "GetAngleBetweenPoints",
            "CGeneral::GetAngleBetweenPoints",
            0x53CEA0,
            Cdecl,
            St0,
        )
        .args(vec![A::F, A::F, A::F, A::F])
        .callees(&[0x53CBE0])
        .route(FPATAN)
        .rust(|c| st0(g::get_angle_between_points_x87(c.f(0), c.f(1), c.f(2), c.f(3)))),
        // ------------------------------------------------------------------ CMatrix: copies and RW
        b(
            "CMatrixUpdateMatrix",
            "CMatrix::UpdateMatrix",
            0x59AD20,
            Thiscall,
            Void,
        )
        .regions(vec![mat(), rwmat()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| {
            let mut m = c.mat(0);
            m.update_matrix(&c.rw(1));
            c.set_mat(0, &m);
            ExpRet::Void
        }),
        b(
            "CMatrixUpdateRwMatrix",
            "CMatrix::UpdateRwMatrix",
            0x59AD70,
            Thiscall,
            Void,
        )
        .regions(vec![mat(), rwmat()])
        .args(vec![A::P(0), A::P(1)])
        .callees(&[RW_MATRIX_UPDATE])
        .rust(|c| {
            let mut r = c.rw(1);
            c.mat(0).update_rw_matrix(&mut r);
            c.set_rw(1, &r);
            ExpRet::Void
        }),
        b(
            "CMatrixCopyToRwMatrix",
            "CMatrix::CopyToRwMatrix",
            0x59B8B0,
            Thiscall,
            Eax,
        )
        .regions(vec![mat(), rwmat()])
        .args(vec![A::P(0), A::P(1)])
        .callees(&[RW_MATRIX_UPDATE])
        .rust(|c| {
            let mut r = c.rw(1);
            c.mat(0).copy_to_rw_matrix(&mut r);
            c.set_rw(1, &r);
            eax(c, 1)
        }),
        b(
            "CMatrixCopyOnlyMatrix",
            "CMatrix::CopyOnlyMatrix",
            0x59ADD0,
            Thiscall,
            Void,
        )
        .regions(vec![mat(), mat()])
        .args(vec![A::P(0), A::P(1)])
        .rust(|c| {
            let mut m = c.mat(0);
            m.copy_only_matrix(&c.mat(1));
            c.set_mat(0, &m);
            ExpRet::Void
        }),
        b("CMatrixUpdate", "CMatrix::Update", 0x59BB60, Thiscall, Void)
            .regions(vec![mat_att_always(1), rwmat()])
            .args(vec![A::P(0)])
            .rust(|c| {
                let mut m = c.mat(0);
                m.update(&c.rw(1));
                c.set_mat(0, &m);
                ExpRet::Void
            }),
        b("CMatrixUpdateRW", "CMatrix::UpdateRW", 0x59BBB0, Thiscall, Void)
            .regions(vec![mat_att(1), rwmat()])
            .args(vec![A::P(0)])
            .callees(&[0x59AD70, RW_MATRIX_UPDATE])
            .rust(|c| {
                let m = c.mat(0);
                let mut r = c.rw(1);
                m.update_rw(Some(&mut r));
                c.set_rw(1, &r);
                ExpRet::Void
            }),
        b("CMatrixAssign", "CMatrix::operator=", 0x59BBC0, Thiscall, Eax)
            .regions(vec![mat_att(2), mat(), rwmat()])
            .args(vec![A::P(0), A::P(1)])
            .callees(&[0x59AD70, RW_MATRIX_UPDATE])
            .rust(|c| {
                let mut m = c.mat(0);
                let mut r = c.rw(2);
                m.assign(&c.mat(1), Some(&mut r));
                c.set_mat(0, &m);
                c.set_rw(2, &r);
                eax(c, 0)
            }),
        b("CMatrixDetach", "CMatrix::Detach", 0x59ACF0, Thiscall, Void)
            .regions(vec![mat_att(1), rwmat()])
            .args(vec![A::P(0)])
            .rust(|c| {
                let mut m = c.mat(0);
                m.detach(|_| panic!("generated inputs never take the RwMatrixDestroy path"));
                c.set_mat(0, &m);
                ExpRet::Void
            }),
        b("CMatrixAttach", "CMatrix::Attach", 0x59BD10, Thiscall, Void)
            .regions(vec![mat_att(1), rwmat(), rwmat()])
            .args(vec![A::P(0), A::P(2), A::I(g_bool)])
            .rust(|c| {
                let mut m = c.mat(0);
                let a = c.addr(2);
                m.attach(a, &c.rw(2), c.i(2) as u8, |_| {
                    panic!("generated inputs never take the RwMatrixDestroy path")
                });
                c.set_mat(0, &m);
                ExpRet::Void
            }),
        // ------------------------------------------------------------------ CMatrix: arithmetic
        b("CMatrixAddAssign", "CMatrix::operator+=", 0x59ADF0, Thiscall, Eax)
            .regions(vec![mat(), mat()])
            .args(vec![A::P(0), A::P(1)])
            .alias(1, 0, 20)
            .rust(|c| {
                let mut m = c.mat(0);
                let r = c.mat(c.ar(1));
                m.add_assign(&r);
                c.set_mat(0, &m);
                eax(c, 0)
            }),
        b(
            "CMatrixMulMat",
            "operator*(const CMatrix&, const CMatrix&)",
            0x59BE30,
            Cdecl,
            Eax,
        )
        .regions(vec![mat(), mat(), mat()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .ignore_mat_pads(0)
        .rust(|c| {
            let r = mx::mul_matrix(&c.mat(1), &c.mat(2));
            c.set_mat(0, &r);
            eax(c, 0)
        }),
        b(
            "CMatrixAdd",
            "operator+(const CMatrix&, const CMatrix&)",
            0x59BFA0,
            Cdecl,
            Eax,
        )
        .regions(vec![mat(), mat(), mat()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .ignore_mat_pads(0)
        .rust(|c| {
            let r = mx::add_matrix(&c.mat(1), &c.mat(2));
            c.set_mat(0, &r);
            eax(c, 0)
        }),
        b("CMatrixMulAssign", "CMatrix::operator*=", 0x411A80, Thiscall, Eax)
            .regions(vec![mat_att(2), mat(), rwmat()])
            .args(vec![A::P(0), A::P(1)])
            .callees(&[0x59BE30, 0x59BBC0, 0x59ACD0, 0x59AD70, RW_MATRIX_UPDATE])
            .ignore_mat_pads(0)
            .rust(|c| {
                let mut m = c.mat(0);
                let mut r = c.rw(2);
                m.mul_assign(&c.mat(1), Some(&mut r));
                c.set_mat(0, &m);
                c.set_rw(2, &r);
                eax(c, 0)
            }),
        b(
            "CMatrixInvert1",
            "Invert(CMatrix& src, CMatrix& dst)",
            0x59B920,
            Cdecl,
            Eax,
        )
        .regions(vec![mat(), mat()])
        .args(vec![A::P(0), A::P(1)])
        .alias(1, 0, 20)
        .rust(|c| {
            if c.aliased {
                let mut m = c.mat(0);
                mx::invert_in_place(&mut m);
                c.set_mat(0, &m);
                eax(c, 0)
            } else {
                let mut d = c.mat(1);
                mx::invert_into(&c.mat(0), &mut d);
                c.set_mat(1, &d);
                eax(c, 1)
            }
        }),
        b("CMatrixInvert2", "Invert(const CMatrix&)", 0x59BDD0, Cdecl, Eax)
            .regions(vec![mat(), mat()])
            .args(vec![A::P(0), A::P(1)])
            .callees(&[0x59B920])
            .ignore_mat_pads(0)
            .rust(|c| {
                let r = mx::invert(&c.mat(1));
                c.set_mat(0, &r);
                eax(c, 0)
            }),
        b("CMatrixScale", "CMatrix::Scale", 0x459350, Thiscall, Void)
            .regions(vec![mat()])
            .args(vec![A::P(0), A::F])
            .rust(|c| {
                let mut m = c.mat(0);
                m.scale(c.f(1));
                c.set_mat(0, &m);
                ExpRet::Void
            }),
        b(
            "CMatrixReorthogonalise",
            "CMatrix::Reorthogonalise",
            0x59B6A0,
            Thiscall,
            Void,
        )
        .regions(vec![mat()])
        .args(vec![A::P(0)])
        .rust(|c| {
            let mut m = c.mat(0);
            m.reorthogonalise();
            c.set_mat(0, &m);
            ExpRet::Void
        }),
        b(
            "CMatrixForceUpVector",
            "CMatrix::ForceUpVector",
            0x59B7E0,
            Thiscall,
            Void,
        )
        .regions(vec![mat()])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .rust(|c| {
            let mut m = c.mat(0);
            m.force_up_vector(c.f(1), c.f(2), c.f(3));
            c.set_mat(0, &m);
            ExpRet::Void
        }),
        b(
            "CMatrixConvertToEulerAngles",
            "CMatrix::ConvertToEulerAngles",
            0x59A840,
            Thiscall,
            Void,
        )
        .regions(vec![mat(), out1(), out1(), out1()])
        .args(vec![A::P(0), A::P(1), A::P(2), A::P(3), A::I(g_euler_flags)])
        .route(FPATAN)
        .rust(|c| {
            let (x, y, z) = c.mat(0).convert_to_euler_angles(c.i(4));
            c.set_fl(1, 0, x);
            c.set_fl(2, 0, y);
            c.set_fl(3, 0, z);
            ExpRet::Void
        }),
        b(
            "CMatrixConvertFromEulerAngles",
            "CMatrix::ConvertFromEulerAngles",
            0x59AA40,
            Thiscall,
            Void,
        )
        .regions(vec![mat()])
        .args(vec![A::P(0), A::F, A::F, A::F, A::I(g_euler_flags)])
        .route(FSIN)
        .rust(|c| {
            let mut m = c.mat(0);
            m.convert_from_euler_angles(c.f(1), c.f(2), c.f(3), c.i(4));
            c.set_mat(0, &m);
            ExpRet::Void
        }),
        // ------------------------------------------------------------------ CCompressedMatrixNotAligned
        b(
            "CCompressedMatrixDecompress",
            "CCompressedMatrixNotAligned::DecompressIntoFullMatrix",
            0x59B9F0,
            Thiscall,
            Void,
        )
        .regions(vec![compressed(), mat()])
        .args(vec![A::P(0), A::P(1)])
        .callees(&[0x59C730, 0x59B6A0])
        .rust(|c| {
            let cm = read_compressed(c, 0);
            let mut m = c.mat(1);
            cm.decompress_into_full_matrix(&mut m);
            c.set_mat(1, &m);
            ExpRet::Void
        }),
        b(
            "CCompressedMatrixCompress",
            "CCompressedMatrixNotAligned::CompressFromFullMatrix",
            0x59BAD0,
            Thiscall,
            Void,
        )
        .regions(vec![compressed(), mat()])
        .args(vec![A::P(0), A::P(1)])
        .callees(&[FTOL])
        .rust(|c| {
            let mut cm = read_compressed(c, 0);
            cm.compress_from_full_matrix(&c.mat(1));
            c.set_vec(0, cm.pos);
            for k in 0..3 {
                c.set_byte(0, 0xC + k, cm.right[k] as u8);
                c.set_byte(0, 0xF + k, cm.forward[k] as u8);
            }
            ExpRet::Void
        }),
    ];
    v.extend(matrix_setters());
    v.extend(crate::list_specs::all());
    v
}

fn out_vec() -> Region {
    raw(3)
}

fn compressed() -> Region {
    Region {
        words: vec![W::F, W::F, W::F, W::R, W::R],
        kind: Kind::Any,
    }
}

fn read_compressed(c: &Ctx, r: usize) -> CCompressedMatrixNotAligned {
    let b = |o: usize| c.byte(r, o) as i8;
    CCompressedMatrixNotAligned {
        pos: c.vec(r),
        right: [b(0xC), b(0xD), b(0xE)],
        forward: [b(0xF), b(0x10), b(0x11)],
        pad: [c.byte(r, 0x12), c.byte(r, 0x13)],
    }
}

fn matrix_setters() -> Vec<Spec> {
    vec![
        mat_setter(
            "CMatrixSetUnity",
            "CMatrix::SetUnity",
            0x59AE70,
            0,
            Route::L1,
            |m, _| m.set_unity(),
        ),
        mat_setter(
            "CMatrixResetOrientation",
            "CMatrix::ResetOrientation",
            0x59AEA0,
            0,
            Route::L1,
            |m, _| m.reset_orientation(),
        ),
        mat_setter(
            "CMatrixSetScale",
            "CMatrix::SetScale(float)",
            0x59AED0,
            1,
            Route::L1,
            |m, f| m.set_scale(f[0]),
        ),
        mat_setter(
            "CMatrixSetScaleXYZ",
            "CMatrix::SetScale(float, float, float)",
            0x59AF00,
            3,
            Route::L1,
            |m, f| m.set_scale_xyz(f[0], f[1], f[2]),
        ),
        mat_setter(
            "CMatrixSetTranslate",
            "CMatrix::SetTranslate",
            0x59AF40,
            3,
            Route::L1,
            |m, f| m.set_translate(f[0], f[1], f[2]),
        ),
        mat_setter(
            "CMatrixSetTranslateOnly",
            "CMatrix::SetTranslateOnly",
            0x59AF80,
            3,
            Route::L1,
            |m, f| m.set_translate_only(f[0], f[1], f[2]),
        ),
        mat_setter(
            "CMatrixSetRotateXOnly",
            "CMatrix::SetRotateXOnly",
            0x59AFA0,
            1,
            FSIN,
            |m, f| m.set_rotate_x_only(f[0]),
        ),
        mat_setter(
            "CMatrixSetRotateYOnly",
            "CMatrix::SetRotateYOnly",
            0x59AFE0,
            1,
            FSIN,
            |m, f| m.set_rotate_y_only(f[0]),
        ),
        mat_setter(
            "CMatrixSetRotateZOnly",
            "CMatrix::SetRotateZOnly",
            0x59B020,
            1,
            FSIN,
            |m, f| m.set_rotate_z_only(f[0]),
        ),
        mat_setter(
            "CMatrixSetRotateX",
            "CMatrix::SetRotateX",
            0x59B060,
            1,
            FSIN,
            |m, f| m.set_rotate_x(f[0]),
        ),
        mat_setter(
            "CMatrixSetRotateY",
            "CMatrix::SetRotateY",
            0x59B0A0,
            1,
            FSIN,
            |m, f| m.set_rotate_y(f[0]),
        ),
        mat_setter(
            "CMatrixSetRotateZ",
            "CMatrix::SetRotateZ",
            0x59B0E0,
            1,
            FSIN,
            |m, f| m.set_rotate_z(f[0]),
        ),
        mat_setter(
            "CMatrixSetRotate",
            "CMatrix::SetRotate(float, float, float)",
            0x59B120,
            3,
            FSIN,
            |m, f| m.set_rotate(f[0], f[1], f[2]),
        ),
        mat_setter("CMatrixRotateX", "CMatrix::RotateX", 0x59B1E0, 1, FSIN, |m, f| {
            m.rotate_x(f[0])
        }),
        mat_setter("CMatrixRotateY", "CMatrix::RotateY", 0x59B2C0, 1, FSIN, |m, f| {
            m.rotate_y(f[0])
        }),
        mat_setter("CMatrixRotateZ", "CMatrix::RotateZ", 0x59B390, 1, FSIN, |m, f| {
            m.rotate_z(f[0])
        }),
        mat_setter("CMatrixRotate", "CMatrix::Rotate", 0x59B460, 3, FSIN, |m, f| {
            m.rotate(f[0], f[1], f[2])
        }),
    ]
}
