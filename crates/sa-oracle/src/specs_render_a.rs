//! L1 specs for PORT-RENDER (`sa_render`): helper A modules (the pure `CCullZones` leaf and the
//! register-preserving `CWeather::FindWeatherTypesList`, whose `WeatherRegion` read is patched to a
//! scratch region and mirrored into sa-render's host global).

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
use sa_render::{cullzones, weather};

/// Scratch address of region 0 (the `B::layout` rule: regions start at scratch +0x10).
const REGION0: u32 = SCRATCH + 0x10;

/// Keeps the region's word a small signed value (-2..7) in most cases.
fn small_region(v: &mut [u32], l: &Layout) {
    if let Some(s) = l.word_slot[0][0] {
        let mut g = Rng::new(u64::from(v[s]) ^ 0x5EA7_4E12);
        if g.below(100) < 90 {
            v[s] = (v[s] & 0xFFFF_0000) | ((g.below(10) as i32 - 2) as u16 as u32);
        }
    }
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "CullZonesIsPointWithinArbitraryArea",
            "CCullZones::IsPointWithinArbitraryArea",
            0x72DDE0,
            Cdecl,
            Al,
        )
        .args(vec![A::F; 10])
        .rust(|c| {
            ExpRet::Al(cullzones::is_point_within_arbitrary_area(
                c.f(0),
                c.f(1),
                c.f(2),
                c.f(3),
                c.f(4),
                c.f(5),
                c.f(6),
                c.f(7),
                c.f(8),
                c.f(9),
            ))
        }),
        // the caller CWeather::Update (0x72B916) keeps ECX / EDX across the call (VER-2)
        b(
            "WeatherFindWeatherTypesList",
            "CWeather::FindWeatherTypesList",
            0x72A520,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(1)])
        .fix(|v, l| small_region(v, l))
        .patch(vec![(0x72A523, REGION0.to_le_bytes().to_vec())])
        .preserve(PRESERVE_ECX | PRESERVE_EDX)
        .rust(|c| {
            let _g = crate::host_globals();
            weather::WEATHER_REGION.set(c.u(0, 0) as u16 as i16);
            ExpRet::Eax(weather::find_weather_types_list().addr() as u32)
        }),
    ]
}
