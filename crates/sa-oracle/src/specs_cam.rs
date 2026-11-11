//! L1 specs for PORT-CAM (`sa_cam`): the pure `CCurves` leaves, `CCam::WrapAngle`, and the `CCamera`
//! entries whose hooks preserve a register (VER-2 preserve specs).
//!
//! The `CCamera` specs put only the window of the camera the function touches in scratch (`this`
//! points before scratch, as in the CCM specs). Where the touched fields lie further apart than the
//! scratch image (`GetFadingDirection`: +0x51 and +0xC30), the displacement of the far field is
//! patched to a word inside the window and the Rust side gets a host camera with the fields copied
//! in. Globals the original reads (`CTimer::m_snTimeInMilliseconds`, `RsGlobal.maximumWidth/Height`)
//! are patched to scratch words; the Rust side gets the same values in the host global (or in a
//! fixed host mapping of the game address, [`crate::hosted::fixed_map`]).

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::math_specs::FSIN_REASON;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Route, Spec};
use sa_cam::{cam, camctl, camera, camfx, curves};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
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

/// Guest address of region `r` of a spec with regions of `words` dwords (the `B::layout` rule).
fn g_addr(words: &[usize], r: usize) -> u32 {
    let mut o = 0x10usize;
    for w in &words[..r] {
        o += (4 * w + 0x10 + 0xF) & !0xF;
    }
    SCRATCH + o as u32
}

/// Patches the absolute operand (the last four bytes of the `len`-byte instruction at `at`) to `to`.
fn operand(sites: &[(u32, u32)], to: u32) -> Vec<(u32, Vec<u8>)> {
    sites
        .iter()
        .map(|&(at, len)| (at + len - 4, to.to_le_bytes().to_vec()))
        .collect()
}

fn hash(v: &[u32]) -> u32 {
    v.iter()
        .fold(0x811C_9DC5u32, |h, &w| (h ^ w).wrapping_mul(0x0100_0193))
}

/// `CCamera::GetFadingDirection`: +0x51 (window word 0 holds +0x50..+0x53) and +0xC30, patched to +0x60.
const FADE_BASE: u32 = 0x50;
const FADE_WORDS: usize = 6;
/// `CCamera::GetScreenRect`: window +0x3C..+0x148 (word 0 holds +0x3D, the last one +0x144).
const RECT_BASE: u32 = 0x3C;
const RECT_WORDS: [usize; 3] = [67, 4, 2];
/// `RsGlobal.maximumWidth` (0xC17044) and `maximumHeight` (0xC17048) operand sites in GetScreenRect.
const RECT_W_SITES: [(u32, u32); 1] = [(0x50AB5B, 6)];
const RECT_H_SITES: [(u32, u32); 5] = [
    (0x50AB6B, 5),
    (0x50AB89, 6),
    (0x50ABA0, 5),
    (0x50ABA5, 6),
    (0x50ABE4, 6),
];
/// `CCamera::ProcessVector{Track,Move}Linear()`: window +0xC5C..+0xCF0.
const LIN_BASE: u32 = 0xC5C;
const LIN_WORDS: [usize; 2] = [37, 1];
/// `CCamera::InitCameraVehicleTweaks`: window +0xCF4..+0xD58.
const TWEAK_BASE: u32 = 0xCF4;

/// The guest `this` with window region 0 at `this + base`.
const fn k_this(base: u32) -> A {
    A::K((SCRATCH + 0x10).wrapping_sub(base))
}

/// The host `this` for region 0 at `this + base`.
fn this_at(c: &mut Ctx, base: u32) -> GPtr<u8> {
    tp::<u8>(c, 0).byte_add(-(base as isize))
}

/// The ProcessVector*Linear window: times and vectors are floats, flags and outputs raw words.
fn lin_window() -> Region {
    let mut w = vec![W::R; LIN_WORDS[0]];
    for i in (0..8).chain(24..32) {
        w[i] = W::F;
    }
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// The FSIN reason for the ProcessVector*Linear specs: their callee runs FSIN on the eased path.
const LIN_REASON: &str = "FSIN in the callee (ProcessVector*Linear(float), eased path); the inputs keep \
     the ease byte clear, so the Unicorn verdict holds as well, but the static scan routes it to the CPU";

/// The eased variants: the ease byte set, so the callee runs FSIN (X87-2: sa-math's 64-bit FSIN model).
const LIN_EASED_REASON: &str = "FSIN in the callee (ProcessVector*Linear(float), eased path: the ease \
     weight sin(x) + 1 cancels near x = 3 pi / 2, exposing FSIN's 64-bit result)";

/// Clears the ease byte (or sets it, for the eased variants) (`+0xC7C` / `+0xCDC`, window words 8 and 32): the eased ratio runs FSIN,
/// and on the real CPU the port of the callee differs there (the ease weight `sin(x) + 1` near
/// x = 3 pi / 2, where the cancellation exposes FSIN's last bits: 29091 / 28912 of 1.06M cases,
/// e.g. weight bits 0x24240000 vs 0x24800000). That is a finding against 0x50D350 / 0x50D430
/// (state `implemented`, no spec of their own), not against these preserve specs. Half the cases
/// put the time just before the end time (the interpolation path), when that is a representable
/// millisecond count.
fn lin_fix(v: &mut [u32], l: &Layout, end_word: usize, eased: bool) {
    let w = |i: usize| l.word_slot[0][i].unwrap();
    for i in [8, 32] {
        v[w(i)] = (v[w(i)] & !0xFF) | u32::from(eased);
    }
    let h = hash(v);
    let end = f32::from_bits(v[w(end_word)]);
    if h & 1 == 0 && end.is_finite() && (1.0..4.0e9).contains(&end) {
        let t = l.word_slot[1][0].unwrap();
        v[t] = (end as u32).wrapping_sub((h >> 1) % 3000);
    }
}

fn lin_spec(
    key: &'static str,
    name: &'static str,
    addr: u32,
    callee: &'static [u32],
    end_word: usize,
    eased: bool,
) -> Spec {
    let t = g_addr(&LIN_WORDS, 1);
    b(key, name, addr, Thiscall, Void)
        .regions(vec![lin_window(), raw(1)])
        .args(vec![k_this(LIN_BASE)])
        .callees(callee)
        .patch(operand(&[(addr + 1, 5), (addr + 6, 6)], t))
        .fix(move |v, l| lin_fix(v, l, end_word, eased))
        .route(Route::L2 {
            reason: if eased { LIN_EASED_REASON } else { LIN_REASON },
        })
        // the callers keep EDX across the call (VER-2)
        .preserve(PRESERVE_EDX)
        .rust(move |c| {
            let _g = crate::host_globals();
            sa_core::timer::TIME_MS.set(c.u(1, 0));
            let this = this_at(c, LIN_BASE);
            if addr == 0x516440 {
                camfx::process_vector_track_linear(this);
            } else {
                camfx::process_vector_move_linear(this);
            }
            ExpRet::Void
        })
}

fn cam_specs() -> Vec<Spec> {
    let rect_dims = g_addr(&RECT_WORDS, 2);
    let mut rect_patch = operand(&RECT_W_SITES, rect_dims);
    rect_patch.extend(operand(&RECT_H_SITES, rect_dims + 4));
    vec![
        b(
            "CameraGetFadingDirection",
            "CCamera::GetFadingDirection",
            0x50ADF0,
            Thiscall,
            Eax,
        )
        .regions(vec![raw(FADE_WORDS)])
        .args(vec![k_this(FADE_BASE)])
        // `cmp word [ecx+0xC30], 1` reads +0x60 instead (inside the window)
        .patch(vec![(0x50AE02, 0x60u32.to_le_bytes().to_vec())])
        .preserve(PRESERVE_EDX)
        .rust(|c| {
            let mut cam = vec![0u8; 0xC34];
            cam[0x51] = c.byte(0, 1);
            cam[0xC30..0xC32].copy_from_slice(&c.u(0, 4).to_le_bytes()[..2]);
            let this = GPtr::from_raw(cam.as_mut_ptr());
            ExpRet::Eax(camera::get_fading_direction(this) as u32)
        }),
        b(
            "CameraGetScreenRect",
            "CCamera::GetScreenRect",
            0x50AB50,
            Thiscall,
            Void,
        )
        .regions(vec![
            {
                let mut w = vec![W::R; RECT_WORDS[0]];
                w[RECT_WORDS[0] - 1] = W::F;
                Region {
                    words: w,
                    kind: Kind::Any,
                }
            },
            raw(RECT_WORDS[1]),
            raw(RECT_WORDS[2]),
        ])
        .args(vec![k_this(RECT_BASE), A::P(1)])
        .patch(rect_patch)
        .fix(|v, l| {
            // mostly realistic screen sizes, sometimes any int
            let h = hash(v);
            if !h.is_multiple_of(4) {
                v[l.word_slot[2][0].unwrap()] = 320 + (h >> 2) % 3600;
                v[l.word_slot[2][1].unwrap()] = 200 + (h >> 14) % 2200;
            }
        })
        .preserve(PRESERVE_ECX)
        .rust(|c| {
            let _g = crate::host_globals();
            let rs = crate::hosted::fixed_map(0xC17000, 0x1000);
            // SAFETY: the page 0xC17000 is mapped by fixed_map and held under host_globals.
            unsafe {
                rs.add(0x44).cast::<u32>().write(c.u(2, 0));
                rs.add(0x48).cast::<u32>().write(c.u(2, 1));
            }
            let (this, rect) = (this_at(c, RECT_BASE), tp::<u8>(c, 1));
            camctl::get_screen_rect(this, rect);
            ExpRet::Void
        }),
        lin_spec(
            "CameraProcessVectorTrackLinear",
            "CCamera::ProcessVectorTrackLinear-wo-ratio",
            0x516440,
            &[0x50D350],
            1,
            false,
        ),
        lin_spec(
            "CameraProcessVectorTrackLinearEased",
            "CCamera::ProcessVectorTrackLinear-wo-ratio",
            0x516440,
            &[0x50D350],
            1,
            true,
        ),
        lin_spec(
            "CameraProcessVectorMoveLinear",
            "CCamera::ProcessVectorMoveLinear-wo-ratio",
            0x5164A0,
            &[0x50D430],
            25,
            false,
        ),
        lin_spec(
            "CameraProcessVectorMoveLinearEased",
            "CCamera::ProcessVectorMoveLinear-wo-ratio",
            0x5164A0,
            &[0x50D430],
            25,
            true,
        ),
        b(
            "CameraInitCameraVehicleTweaks",
            "CCamera::InitCameraVehicleTweaks",
            0x50A3B0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(0x64 / 4)])
        .args(vec![k_this(TWEAK_BASE)])
        .preserve(PRESERVE_ECX)
        .rust(|c| {
            camfx::init_camera_vehicle_tweaks(this_at(c, TWEAK_BASE));
            ExpRet::Void
        }),
    ]
}

pub fn all() -> Vec<Spec> {
    let mut v = cam_specs();
    v.extend(vec![
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
            // the callers keep EDX across the call (VER-2)
            .preserve(sa_core::abi::PRESERVE_EDX)
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
    ]);
    v
}
