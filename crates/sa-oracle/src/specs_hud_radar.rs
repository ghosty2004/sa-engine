//! L1 specs for PORT-HUD (`sa_hud`, see `docs/arch/hud-port.md`): CRadar leaves. Globals the
//! originals read are patched with fixed values (one spec per menu-flag state where it matters); the
//! ports' `_with` / `_in` variants get the same values as parameters.

use std::sync::LazyLock;

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_hud::radar;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x4AD_A12;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// `CRadar::ms_RadarTrace` as patched for the handle specs: counter `i % 5 + 1`, in-use bit set for
/// two entries of three (flags byte `i * 7` with bit 1 forced).
static TRACES: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut t = vec![0u8; 175 * 0x28];
    for i in 0..175usize {
        let e = &mut t[i * 0x28..(i + 1) * 0x28];
        e[0x14..0x16].copy_from_slice(&((i % 5 + 1) as u16).to_le_bytes());
        let f = (i * 7) as u8;
        e[0x25] = if i % 3 == 2 { f & !2 } else { f | 2 };
        e[0x26] = (i * 13) as u8;
    }
    t
});

const TRACE_ADDR: u32 = 0xBA86F0;
const MENU_FLAG: u32 = 0xBA67A1;
const MENU_ZOOM: u32 = 0xBA67AC;

/// A handle: an index below 175 with a counter in 0..=6 (65 %), any counter, or -1.
fn g_handle(r: &mut Rng) -> u32 {
    let i = r.below(175);
    match r.below(20) {
        0..=12 => (r.below(7) << 16) | i,
        13..=17 => (r.next_u32() & 0xFFFF_0000) | i,
        _ => u32::MAX,
    }
}

fn f32b(v: f32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

const ORIGIN: [f32; 2] = [1234.5, -987.25];
const RANGE: f32 = 350.0;
const ANGLE: f32 = 0.7;
const MAP_ORIGIN: [f32; 2] = [320.0, 224.5];
const MAP_ZOOM: f32 = 140.0;
const SCREEN: [i32; 2] = [1920, 1080];

/// Tile coordinates: 70 % in [-3, 14].
fn tiles(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    for regs in &lay.word_slot {
        for s in regs.iter().flatten() {
            if r.below(100) < 70 {
                v[*s] = (r.below(18) as i32 - 3) as u32;
            }
        }
    }
}

fn g_tile(r: &mut Rng) -> u32 {
    if r.chance(70) {
        (r.below(18) as i32 - 3) as u32
    } else {
        r.next_u32()
    }
}

fn limit(key: &'static str, map: bool) -> Spec {
    b(key, "CRadar::LimitRadarPoint", 0x5832F0, Cdecl, St0)
        .regions(vec![vec2()])
        .args(vec![A::P(0)])
        .patch(vec![(MENU_FLAG, vec![u8::from(map)])])
        .reads(vec![(MENU_FLAG, MENU_FLAG + 1)])
        .rust(move |c| st0(radar::limit_radar_point_with(map, tp(c, 0))))
}

fn screen(key: &'static str, map: bool) -> Spec {
    let mut p = SCREEN[0].to_le_bytes().to_vec();
    p.extend_from_slice(&SCREEN[1].to_le_bytes());
    let mut z = f32b(MAP_ZOOM);
    z.extend(f32s(&MAP_ORIGIN));
    b(
        key,
        "CRadar::TransformRadarPointToScreenSpace",
        0x583480,
        Cdecl,
        Void,
    )
    .preserve(sa_core::abi::PRESERVE_EDX)
    .regions(vec![vec2(), vec2()])
    .args(vec![A::P(0), A::P(1)])
    .alias(1, 0, 20)
    .patch(vec![
        (MENU_FLAG, vec![u8::from(map)]),
        (MENU_ZOOM, z),
        (0xC17044, p),
    ])
    .reads(vec![
        (MENU_FLAG, MENU_FLAG + 1),
        (MENU_ZOOM, MENU_ZOOM + 12),
        (0xC17044, 0xC1704C),
    ])
    .rust(move |c| {
        let (o, i) = (c.ar(0), c.ar(1));
        radar::transform_radar_point_to_screen_space_with(
            tp(c, o),
            tp(c, i),
            map,
            MAP_ZOOM,
            MAP_ORIGIN,
            SCREEN,
        );
        ExpRet::Void
    })
}

fn alpha(key: &'static str, map: bool) -> Spec {
    b(key, "CRadar::CalculateBlipAlpha", 0x583420, Cdecl, Al)
        .callees(&[0x821B40])
        .args(vec![A::F])
        .patch(vec![(MENU_FLAG, vec![u8::from(map)])])
        .reads(vec![(MENU_FLAG, MENU_FLAG + 1)])
        .rust(move |c| ExpRet::Al(radar::calculate_blip_alpha_with(map, c.f(0))))
}

pub fn all() -> Vec<Spec> {
    let (sin, cos) = (ANGLE.sin(), ANGLE.cos());
    vec![
        b(
            "HudRadarGetActualBlipArrayIndex",
            "CRadar::GetActualBlipArrayIndex",
            0x582870,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_handle)])
        .patch(vec![(TRACE_ADDR, TRACES.clone())])
        .reads(vec![(TRACE_ADDR, TRACE_ADDR + 175 * 0x28)])
        .rust(|c| {
            let base = GPtr::from_raw(TRACES.as_ptr().cast_mut());
            ExpRet::Eax(radar::get_actual_blip_array_index_in(base, c.i(0) as i32) as u32)
        }),
        limit("HudRadarLimitRadarPoint", false),
        limit("HudRadarLimitRadarPointMap", true),
        b(
            "HudRadarTransformRealWorldPointToRadarSpace",
            "CRadar::TransformRealWorldPointToRadarSpace",
            0x583530,
            Cdecl,
            Void,
        )
        .regions(vec![vec2(), vec2()])
        .args(vec![A::P(0), A::P(1)])
        .alias(1, 0, 20)
        .patch(vec![
            (0xBAA248, f32s(&ORIGIN)),
            (0xBA8308, f32s(&[cos, sin])),
            (0xBA8314, f32b(RANGE)),
        ])
        .reads(vec![
            (0xBAA248, 0xBAA250),
            (0xBA8308, 0xBA8310),
            (0xBA8314, 0xBA8318),
        ])
        .rust(move |c| {
            let (o, i) = (c.ar(0), c.ar(1));
            radar::transform_real_world_point_to_radar_space_with(
                tp(c, o),
                tp(c, i),
                ORIGIN,
                RANGE,
                cos,
                sin,
            );
            ExpRet::Void
        }),
        screen("HudRadarTransformRadarPointToScreenSpace", false),
        screen("HudRadarTransformRadarPointToScreenSpaceMap", true),
        alpha("HudRadarCalculateBlipAlpha", false),
        alpha("HudRadarCalculateBlipAlphaMap", true),
        b(
            "HudRadarIsPointInsideRadar",
            "CRadar::IsPointInsideRadar",
            0x584D40,
            Cdecl,
            Al,
        )
        .regions(vec![vec2()])
        .args(vec![A::P(0)])
        .rust(|c| ExpRet::Al(radar::is_point_inside_radar(tp(c, 0)))),
        b(
            "HudRadarClipRadarTileCoords",
            "CRadar::ClipRadarTileCoords",
            0x584B00,
            Cdecl,
            Al,
        )
        .regions(vec![raw(1), raw(1)])
        .args(vec![A::P(0), A::P(1)])
        .alias(1, 0, 10)
        .fix(|v, l| tiles(v, l))
        .rust(|c| {
            let (a, b2) = (c.ar(0), c.ar(1));
            ExpRet::Al(radar::clip_radar_tile_coords(tp(c, a), tp(c, b2)))
        }),
        b(
            "HudRadarGetTextureCorners",
            "CRadar::GetTextureCorners",
            0x584D90,
            Cdecl,
            Void,
        )
        .regions(vec![raw(8)])
        .args(vec![A::I(g_tile), A::I(g_tile), A::P(0)])
        .rust(|c| {
            let (x, y) = (c.i(0) as i32, c.i(1) as i32);
            radar::get_texture_corners(x, y, tp(c, 0));
            ExpRet::Void
        }),
    ]
}
