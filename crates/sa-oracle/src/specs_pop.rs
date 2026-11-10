//! L1 specs for PORT-POP (`sa_pop`, see `docs/arch/pop-port.md`): the pure leaves
//! (`CTrafficLights::FindTrafficLightTypeFromOrientation`, `CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes`,
//! `CPopulation::FindPedRaceFromName`).

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_pop::{carctrl, misc, population};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x90_9A_11;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// Angles: 70 % on a grid of quarter degrees in [-30, 390] (every boundary 60/150/240/330 is hit).
fn angles(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    if r.below(100) >= 70 {
        return;
    }
    for x in v.iter_mut().take(lay.n_float) {
        *x = ((r.below(1681) as i32 - 120) as f32 * 0.25).to_bits();
    }
}

/// Name bytes: 80 % of the words made of letters from "BWIOHbwiohxAZaz" (and NUL).
fn race_letters(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    if r.below(100) >= 80 {
        return;
    }
    const L: &[u8] = b"BWIOHbwiohxAZaz\0@[`{";
    for regs in &lay.word_slot {
        for s in regs.iter().flatten() {
            let mut w = 0u32;
            for k in 0..4 {
                w |= u32::from(L[r.below(L.len() as u32) as usize]) << (8 * k);
            }
            v[*s] = w;
        }
    }
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "PopFindTrafficLightType",
            "CTrafficLights::FindTrafficLightTypeFromOrientation",
            0x49D4D0,
            Cdecl,
            Eax,
        )
        .args(vec![A::F])
        .fix(|v, l| angles(v, l))
        .rust(|c| ExpRet::Eax(misc::find_traffic_light_type_from_orientation(c.f(0)) as u32)),
        b(
            "PopFindSpeedMultiplierWithSpeedFromNodes",
            "CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes",
            0x424130,
            Cdecl,
            St0,
        )
        .args(vec![A::I(|r| {
            if r.chance(70) {
                r.below(4) | if r.chance(20) { 0xFF } else { 0 }
            } else {
                r.next_u32()
            }
        })])
        .rust(|c| st0(carctrl::find_speed_multiplier_with_speed_from_nodes(c.i(0) as u8))),
        b(
            "PopFindPedRaceFromName",
            "CPopulation::FindPedRaceFromName",
            0x5B6D40,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(1)])
        .args(vec![A::P(0)])
        .fix(|v, l| race_letters(v, l))
        .rust(|c| ExpRet::Eax(population::find_ped_race_from_name(tp(c, 0)) as u32)),
    ]
}
