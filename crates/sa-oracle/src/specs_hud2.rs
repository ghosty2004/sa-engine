//! L1 specs for PORT-HUD-2 (`sa_hud`, see `docs/arch/hud-port.md`): the radar polygon clipping leaves
//! `LineRadarBoxCollision` and `ClipRadarPoly` (pure: they read their input vectors and write the output
//! array). Inputs are biased towards the radar square (-1..1) so that crossings, corners and the
//! "square inside the quad" case are frequent.

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_hud::radar_draw;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x4AD_2C1;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// 85 % of the input words of regions `from..` become a float near the square: a multiple of 1/8 in
/// [-4, 4] (exact hits on the edges) or a uniform value in [-3, 3]; the rest keep the generator's value.
fn near_square(v: &mut [u32], lay: &Layout, from: usize) {
    let pct = 85;
    let mut r = Rng::new(seed(v));
    for regs in lay.word_slot.iter().skip(from) {
        for s in regs.iter().flatten() {
            let k = r.below(100);
            if k >= pct {
                continue;
            }
            match k * 100 / pct {
                0..=46 => v[*s] = ((r.below(65) as f32 - 32.0) / 8.0).to_bits(),
                _ => v[*s] = ((r.below(1 << 20) as f32 / (1 << 20) as f32) * 6.0 - 3.0).to_bits(),
            }
        }
    }
}

/// Emulated address of the first region, and of `ClipRadarPoly`'s `inside[-1]` local (fixed harness stack).
const EMU_REGIONS: u32 = 0x1000_0000;
const EMU_INSIDE_M1: u32 = 0x001E_FFAB;

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

const ORIGIN: [f32; 2] = [1234.5, -987.25];
const RANGE: f32 = 350.0;
const ANGLE: f32 = 0.7;
const MAP_ORIGIN: [f32; 2] = [320.0, 224.5];
const SCREEN: [i32; 2] = [1920, 1080];
const MENU_MAP_LOADED: u32 = 0xBA67C0;
const MENU_MAP_ZOOM: u32 = 0xBA67AC;

/// `LimitToMap` with the map loaded (zoom `zoom`, origin `MAP_ORIGIN`) or not.
fn limit_to_map(key: &'static str, loaded: bool, zoom: f32) -> Spec {
    let mut z = zoom.to_le_bytes().to_vec();
    z.extend(f32s(&MAP_ORIGIN));
    let mut scr = SCREEN[0].to_le_bytes().to_vec();
    scr.extend_from_slice(&SCREEN[1].to_le_bytes());
    b(key, "CRadar::LimitToMap", 0x583350, Cdecl, Void)
        .callees(&[0x821B40])
        .regions(vec![floats(1), floats(1)])
        .args(vec![A::P(0), A::P(1)])
        .patch(vec![
            (MENU_MAP_LOADED, vec![u8::from(loaded)]),
            (MENU_MAP_ZOOM, z),
            (0xC17044, scr),
        ])
        .reads(vec![
            (MENU_MAP_LOADED, MENU_MAP_LOADED + 1),
            (MENU_MAP_ZOOM, MENU_MAP_ZOOM + 12),
            (0xC17044, 0xC1704C),
        ])
        .rust(move |c| {
            let (px, py) = (tp(c, 0), tp(c, 1));
            radar_draw::limit_to_map_with(px, py, loaded, zoom, MAP_ORIGIN, SCREEN);
            ExpRet::Void
        })
}

/// Signalling NaNs among the input floats become quiet: a signalling-NaN case is decided on the real CPU
/// (native backend), whose stack differs from the emulator's, and `ClipRadarPoly`'s `corners[-1]` quirk
/// writes a stack address (the spec models the emulator's).
fn quiet_snans(v: &mut [u32], lay: &Layout) {
    for regs in &lay.word_slot {
        for s in regs.iter().flatten() {
            let w = v[*s];
            if w & 0x7F80_0000 == 0x7F80_0000 && w & 0x7F_FFFF != 0 {
                v[*s] = w | 0x40_0000;
            }
        }
    }
}

pub fn all() -> Vec<Spec> {
    let (sin, cos) = (ANGLE.sin(), ANGLE.cos());
    vec![
        limit_to_map("HudRadarLimitToMap", false, 300.0),
        limit_to_map("HudRadarLimitToMapLoaded", true, 300.0),
        limit_to_map("HudRadarLimitToMapLoadedBig", true, 70000.7),
        b(
            "HudRadarTransformRadarPointToRealWorldSpace",
            "CRadar::TransformRadarPointToRealWorldSpace",
            0x5835A0,
            Cdecl,
            Void,
        )
        .regions(vec![vec2(), vec2()])
        .args(vec![A::P(0), A::P(1)])
        .alias(1, 0, 20)
        .patch(vec![
            (0xBAA248, f32s(&ORIGIN)),
            (0xBA8308, f32s(&[cos, sin])),
            (0xBA8314, f32s(&[RANGE])),
        ])
        .reads(vec![
            (0xBAA248, 0xBAA250),
            (0xBA8308, 0xBA8310),
            (0xBA8314, 0xBA8318),
        ])
        .rust(move |c| {
            let (o, i) = (c.ar(0), c.ar(1));
            radar_draw::transform_radar_point_to_real_world_space_with(tp(c, o), tp(c, i), cos, sin, RANGE, ORIGIN);
            ExpRet::Void
        }),
        b(
            "HudRadarLineRadarBoxCollision",
            "CRadar::LineRadarBoxCollision",
            0x584E00,
            Cdecl,
            Eax,
        )
        .regions(vec![vec2(), vec2(), vec2()])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .fix(|v, l| near_square(v, l, 1))
        .rust(|c| {
            let (o, s, e) = (tp(c, 0), tp(c, 1), tp(c, 2));
            ExpRet::Eax(radar_draw::line_radar_box_collision(o, s, e) as u32)
        }),
        b(
            "HudRadarClipRadarPoly",
            "CRadar::ClipRadarPoly",
            0x585040,
            Cdecl,
            Eax,
        )
        .callees(&[0x584E00])
        .regions(vec![floats(32), floats(8)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| {
            near_square(v, l, 1);
            quiet_snans(v, l);
        })
        .rust(|c| {
            let (o, i) = (tp::<u8>(c, 0), tp(c, 1));
            // The original's `&inside[-1]` in the harness stack, and the emulated address of a slot.
            let emu_out = EMU_REGIONS + c.lay.off[0] as u32;
            let quirk = move |_: u32, slot: GPtr<u8>| (EMU_INSIDE_M1, emu_out + (slot.addr() - o.addr()) as u32);
            ExpRet::Eax(radar_draw::clip_radar_poly_with(o, i, quirk) as u32)
        }),
    ]
}
