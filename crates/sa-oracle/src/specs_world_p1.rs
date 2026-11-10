//! L1 specs for PORT-WORLD part 1 (`sa_world::world`, see `docs/arch/world-port.md`): the pure leaves
//! (zone geometry, `CZoneDef`, `CGarage` point tests, `CPickup` position packing, `CGridRef`, the
//! `CStats` conversions, `CGameLogic` city/line-area tests). Every spec runs the hooked entry on the
//! host on a copy of the scratch image. [`small_ints`] puts the int16 zone fields and the floats on a
//! small common grid in most cases, so the boundaries (equal coordinates) are frequent.

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
use sa_core::mem::GPtr;
use sa_world::world::{common, cworld, entryexit, gamelogic, garages, misc, pickups, stats, zones};

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x5A_57_0D;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// 85 % of the cases: every float on a grid of halves in [-8, 8] and every raw word made of two int16
/// in [-8, 8]; the rest keep the generic mix.
pub fn small_ints(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    if r.below(100) >= 85 {
        return;
    }
    for (i, x) in v.iter_mut().enumerate() {
        if i < lay.n_float {
            *x = ((r.below(33) as i32 - 16) as f32 * 0.5).to_bits();
        }
    }
    for regs in &lay.word_slot {
        for s in regs.iter().flatten() {
            if *s >= lay.n_float {
                let a = (r.below(17) as i32 - 8) as i16 as u16 as u32;
                let b = (r.below(17) as i32 - 8) as i16 as u16 as u32;
                v[*s] = a | (b << 16);
            }
        }
    }
}

fn floats_small(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    if r.below(100) >= 70 {
        return;
    }
    for x in v.iter_mut().take(lay.n_float) {
        *x = ((r.below(33) as i32 - 16) as f32 * 0.5).to_bits();
    }
}

/// Coordinates spread over the map (±3500) in 70 % of the cases.
fn map_coords(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    if r.below(100) >= 70 {
        return;
    }
    for x in v.iter_mut().take(lay.n_float) {
        *x = r.uniform(3500.0).to_bits();
    }
}

fn g_small_int(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.next_u32(),
        1 => r.below(200),
        2 => (r.below(400) as i32 - 200) as u32,
        _ => r.below(0x10000),
    }
}

fn g_stat(r: &mut Rng) -> u32 {
    if r.chance(80) {
        r.below(0x140)
    } else {
        r.next_u32()
    }
}

fn g_count(r: &mut Rng) -> u32 {
    if r.chance(90) {
        r.below(9)
    } else {
        (r.below(8) as i32 - 8) as u32
    }
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "TheZonesZoneIsEntirelyContained",
            "CTheZones::ZoneIsEntirelyContainedWithinOtherZone",
            0x572220,
            Cdecl,
            Al,
        )
        .regions(vec![raw(8), raw(8)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| small_ints(v, l))
        .rust(|c| {
            ExpRet::Al(zones::zone_is_entirely_contained_within_other_zone(
                tp(c, 0),
                tp(c, 1),
            ))
        }),
        b(
            "TheZonesPointLiesWithinZone",
            "CTheZones::PointLiesWithinZone",
            0x572270,
            Cdecl,
            Al,
        )
        .regions(vec![vec3(), raw(8)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| small_ints(v, l))
        .rust(|c| ExpRet::Al(zones::point_lies_within_zone(tp(c, 0), tp(c, 1)))),
        b(
            "TheZonesCalc2DDistanceBetween2Zones",
            "CTheZones::Calc2DDistanceBetween2Zones",
            0x5725B0,
            Cdecl,
            St0,
        )
        .regions(vec![raw(8), raw(8)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| small_ints(v, l))
        .rust(|c| st0(zones::calc_2d_distance_between_2_zones(tp(c, 0), tp(c, 1)))),
        b(
            "ZoneDefIsPointWithin",
            "CZoneDef::IsPointWithin",
            0x72D850,
            Thiscall,
            Al,
        )
        .regions(vec![raw(4)])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .fix(|v, l| small_ints(v, l))
        .rust(|c| {
            let (x, y, z) = (c.f(1), c.f(2), c.f(3));
            ExpRet::Al(zones::zone_def_is_point_within(tp(c, 0), x, y, z))
        }),
        b(
            "GameLogicFindCityClosestToPoint",
            "CGameLogic::FindCityClosestToPoint",
            0x441B70,
            Cdecl,
            Eax,
        )
        .args(vec![A::F, A::F])
        .fix(|v, l| map_coords(v, l))
        .rust(|c| ExpRet::Eax(gamelogic::find_city_closest_to_point(c.f(0), c.f(1)) as u32)),
        b(
            "GameLogicIsPointWithinLineArea",
            "CGameLogic::IsPointWithinLineArea",
            0x4416E0,
            Cdecl,
            Eax,
        )
        .regions(vec![floats(24)])
        .args(vec![A::P(0), A::I(g_count), A::F, A::F])
        .callees(&[0x4138D0])
        .fix(|v, l| floats_small(v, l))
        .budget(200_000)
        .rust(|c| {
            let (n, x, y) = (c.i(1) as i32, c.f(2).to_bits(), c.f(3).to_bits());
            ExpRet::Eax(gamelogic::is_point_within_line_area(tp(c, 0), n, x, y))
        }),
        b(
            "StatsConvertToMins",
            "CStats::ConvertToMins",
            0x559540,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_small_int)])
        .rust(|c| ExpRet::Eax(stats::convert_to_mins(c.i(0) as i32) as u32)),
        b(
            "StatsConvertToSecs",
            "CStats::ConvertToSecs",
            0x559560,
            Cdecl,
            Eax,
        )
        .args(vec![A::I(g_small_int)])
        .rust(|c| ExpRet::Eax(stats::convert_to_secs(c.i(0) as i32) as u32)),
        b("StatsIsStatCapped", "CStats::IsStatCapped", 0x559630, Cdecl, Al)
            .args(vec![A::I(g_stat)])
            .preserve(PRESERVE_ECX)
            .rust(|c| ExpRet::Al(stats::is_stat_capped(c.i(0) as u16))),
        b("StatsIsStatFloat", "CStats::IsStatFloat", 0x558E30, Cdecl, Al)
            .args(vec![A::I(g_stat)])
            .rust(|c| ExpRet::Al(stats::is_stat_float(c.i(0) as u16))),
        b(
            "StatsCheckForThreshold",
            "CStats::CheckForThreshold",
            0x5595F0,
            Cdecl,
            Al,
        )
        .regions(vec![floats(1)])
        .args(vec![A::P(0), A::F])
        .fix(|v, l| {
            let mut r = Rng::new(seed(v));
            if r.chance(70) {
                for x in v.iter_mut().take(l.n_float) {
                    *x = (r.below(400) as f32 - 200.0).to_bits();
                }
            }
        })
        .rust(|c| {
            let t = c.f(1).to_bits();
            ExpRet::Al(stats::check_for_threshold(tp(c, 0), t))
        }),
        b("PickupsIsType345", "thunk_FUN_0156f730", 0x454920, Cdecl, Eax)
            .args(vec![A::I(
                |r| if r.chance(80) { r.below(12) } else { r.next_u32() },
            )])
            .rust(|c| ExpRet::Eax(pickups::is_type_3_4_5(c.i(0) as i32))),
        b("PickupGetPosn", "CPickup::GetPosn", 0x4549A0, Thiscall, Eax)
            .regions(vec![raw(8), raw(3)])
            .args(vec![A::P(0), A::P(1)])
            .rust(|c| {
                pickups::get_posn(tp(c, 0), tp(c, 1));
                ExpRet::Eax(c.addr(1))
            }),
        b("PickupSetPosn", "CPickup::SetPosn", 0x454960, Thiscall, Void)
            .regions(vec![raw(8)])
            .callees(&[0x821B40])
            .args(vec![A::P(0), A::F, A::F, A::F])
            .fix(|v, l| map_coords(v, l))
            .rust(|c| {
                let (x, y, z) = (c.f(1), c.f(2), c.f(3));
                pickups::set_posn(tp(c, 0), x, y, z);
                ExpRet::Void
            }),
        b(
            "GridRefGetGridRefPositionsVec",
            "CGridRef::GetGridRefPositions-vec",
            0x71D5A0,
            Cdecl,
            Void,
        )
        .regions(vec![raw(1), raw(1)])
        .args(vec![A::F, A::F, A::F, A::P(0), A::P(1)])
        .callees(&[0x821B40])
        .fix(|v, l| map_coords(v, l))
        .rust(|c| {
            let (x, y, z) = (c.f(0), c.f(1), c.f(2));
            misc::get_grid_ref_positions_vec(x, y, z, tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
        b(
            "GarageIsPointInsideGarage",
            "CGarage::IsPointInsideGarage-CVector",
            0x448740,
            Thiscall,
            Al,
        )
        .regions(vec![floats(10)])
        .args(vec![A::P(0), A::F, A::F, A::F])
        .preserve(PRESERVE_EDX)
        .fix(|v, l| floats_small(v, l))
        .rust(|c| {
            let (x, y, z) = (c.f(1), c.f(2), c.f(3));
            ExpRet::Al(garages::is_point_inside_garage(tp(c, 0), x, y, z))
        }),
        b(
            "GarageIsPointInsideGarageMargin",
            "CGarage::IsPointInsideGarage-CVector-float",
            0x4487D0,
            Thiscall,
            Al,
        )
        .regions(vec![floats(10)])
        .args(vec![A::P(0), A::F, A::F, A::F, A::F])
        .preserve(PRESERVE_ECX | PRESERVE_EDX)
        .fix(|v, l| floats_small(v, l))
        .rust(|c| {
            let (x, y, z, m) = (c.f(1), c.f(2), c.f(3), c.f(4));
            ExpRet::Al(garages::is_point_inside_garage_margin(tp(c, 0), x, y, z, m))
        }),
        // `FindPlayerWanted`: `PlayerInFocus` (0xB7CD74) and `Players[i].m_pWanted` (0xB7CD9C + i*0x190)
        // read through patched operands from scratch: region 0 = the focus byte (0 or 1), region 1 =
        // `CWorld::Players` from +0 (words 1 and 101 are the two wanted pointers). Ids -1, 0, 1.
        b(
            "WorldFindPlayerWanted",
            "CWorld::FindPlayerWanted",
            0x56E230,
            Cdecl,
            Eax,
        )
        .regions(vec![raw(1), raw(102)])
        .args(vec![A::I(|r| (r.below(3) as i32 - 1) as u32)])
        .patch(vec![
            (0x56E23B, (SCRATCH + 0x10).to_le_bytes().to_vec()),
            (0x56E247, (SCRATCH + 0x30 + 4).to_le_bytes().to_vec()),
        ])
        .fix(|v, l| {
            let s = l.word_slot[0][0].unwrap();
            v[s] &= 1;
        })
        .preserve(PRESERVE_ECX)
        .rust(|c| {
            let _g = crate::host_globals();
            cworld::PLAYER_IN_FOCUS.set(c.byte(0, 0));
            common::PLAYER0_WANTED.set(GPtr::from_addr(c.u(1, 1) as usize));
            common::PLAYER1_WANTED.set(GPtr::from_addr(c.u(1, 101) as usize));
            let r = cworld::find_player_wanted(c.i(0) as i32);
            ExpRet::Eax(r.addr() as u32)
        }),
        // `CPickup::IsVisible`: the camera entity pointer (0xB6F03C) is read from a zero word in scratch
        // (region 2) and the fallback vector (0xB6F02C, an immediate) is region 1.
        b("PickupIsVisible", "CPickup::IsVisible", 0x454C70, Thiscall, Eax)
            .regions(vec![
                raw(8),
                floats(3),
                Region {
                    words: vec![W::C(0)],
                    kind: Kind::Any,
                },
            ])
            .args(vec![A::P(0)])
            .patch(vec![
                (0x454C80, (SCRATCH + 0x60).to_le_bytes().to_vec()),
                (0x454CAA, (SCRATCH + 0x40).to_le_bytes().to_vec()),
            ])
            .fix(|v, l| small_ints(v, l))
            .preserve(PRESERVE_EDX)
            .rust(|c| {
                let _g = crate::host_globals();
                common::CAM_ENTITY.set(GPtr::NULL);
                common::CAM_VECTOR.set([c.fl(1, 0), c.fl(1, 1), c.fl(1, 2)]);
                ExpRet::Eax(pickups::is_visible(tp(c, 0)))
            }),
        b(
            "GaragePlayerArrestedOrDied",
            "CGarage::PlayerArrestedOrDied",
            0x4486C0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(32)])
        .args(vec![A::P(0)])
        .fix(|v, l| {
            // type byte +0x4C and door byte +0x4D small
            let s = l.word_slot[0][0x13].unwrap();
            let mut r = Rng::new(seed(v));
            let ty = r.below(50);
            let door = r.below(7);
            v[s] = (v[s] & 0xFFFF_0000) | ty | (door << 8);
        })
        .preserve(PRESERVE_ECX | PRESERVE_EDX)
        .rust(|c| {
            garages::player_arrested_or_died(tp(c, 0));
            ExpRet::Void
        }),
        // `CEntryExit::GetPositionRelativeToOutsideWorld`: no link (+0x38 = 0; the following word too,
        // for the 64-bit host read); area byte +0x32 random.
        b(
            "EntryExitGetPositionRelativeToOutsideWorld",
            "CEntryExit::GetPositionRelativeToOutsideWorld",
            0x43EA00,
            Thiscall,
            Void,
        )
        .regions(vec![
            Region {
                words: {
                    let mut w = vec![
                        W::R,
                        W::R,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::F,
                        W::R,
                        W::R,
                    ];
                    w.extend([W::C(0), W::C(0)]);
                    w
                },
                kind: Kind::Any,
            },
            vec3(),
        ])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| floats_small(v, l))
        .preserve(PRESERVE_EDX)
        .rust(|c| {
            entryexit::get_position_relative_to_outside_world(tp(c, 0), tp(c, 1));
            ExpRet::Void
        }),
    ]
}
