//! L1 specs for PORT-CAM (`sa_cam`): the pure `CCurves` leaves and `CCam::WrapAngle`.

use crate::call::Conv::*;
use crate::math_specs::FSIN_REASON;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Route, Spec};
use sa_cam::{cam, camctl, curves};
use sa_core::mem::GPtr;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// WrapAngle loops `|a| / 2 pi` times and never ends for +-inf (as the original): keep the float words
/// of region 0 finite and below 1e4 in magnitude.
fn finite_angle(v: &mut [u32], lay: &Layout) {
    for s in lay.word_slot[0].iter().flatten() {
        let a = f32::from_bits(v[*s]);
        if a.is_infinite() || a.abs() >= 1.0e4 {
            v[*s] = (v[*s] & 0x807F_FFFF) | 0x4100_0000;
        }
    }
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "CameraCalculateMirroredMatrix",
            "CCamera::CalculateMirroredMatrix",
            0x50B380,
            Thiscall,
            Void,
        )
        .regions(vec![mat(), raw(16)])
        .args(vec![A::K(0), A::F, A::F, A::F, A::F, A::P(0), A::P(1)])
        .rust(|c| {
            camctl::calculate_mirrored_matrix(GPtr::NULL, c.f(1), c.f(2), c.f(3), c.f(4), tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
        b("CamWrapAngle", "CCam::WrapAngle", 0x509BE0, Cdecl, Void)
            .regions(vec![floats(1)])
            .args(vec![A::P(0)])
            .fix(|v, l| finite_angle(v, l))
            .rust(|c| {
                cam::wrap_angle(tp(c, 0));
                ExpRet::Void
            }),
        b(
            "CurvesDistForLineToCrossOtherLine",
            "CCurves::DistForLineToCrossOtherLine",
            0x43C610,
            Cdecl,
            St0,
        )
        .args(vec![A::F; 8])
        .rust(|c| {
            st0(curves::dist_for_line_to_cross_other_line(
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
            "CurvesCalcCorrectedDist",
            "CCurves::CalcCorrectedDist",
            0x43C880,
            Cdecl,
            St0,
        )
        .regions(vec![out1()])
        .args(vec![A::F, A::F, A::F, A::P(0)])
        .route(Route::L2 { reason: FSIN_REASON })
        .rust(|c| st0(curves::calc_corrected_dist(c.f(0), c.f(1), c.f(2), tp(c, 0)))),
        b(
            "CurvesCalcSpeedVariationInBend",
            "CCurves::CalcSpeedVariationInBend",
            0x43C660,
            Cdecl,
            St0,
        )
        .regions(vec![vec3(), vec3()])
        .args(vec![A::P(0), A::P(1), A::F, A::F, A::F, A::F])
        .callees(&[0x412A30])
        .rust(|c| {
            st0(curves::calc_speed_variation_in_bend(
                tp(c, 0),
                tp(c, 1),
                c.f(2),
                c.f(3),
                c.f(4),
                c.f(5),
            ))
        }),
        b(
            "CurvesCalcSpeedScaleFactor",
            "CCurves::CalcSpeedScaleFactor",
            0x43C710,
            Cdecl,
            St0,
        )
        .regions(vec![vec3(), vec3()])
        .args(vec![A::P(0), A::P(1), A::F, A::F, A::F, A::F])
        .callees(&[0x43C660, 0x412A30])
        .rust(|c| {
            st0(curves::calc_speed_scale_factor(
                tp(c, 0),
                tp(c, 1),
                c.f(2),
                c.f(3),
                c.f(4),
                c.f(5),
            ))
        }),
    ]
}
