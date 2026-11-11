//! `CPopCycle` (PORT-POP). Ported from the 1.0 US bytes; gta-reversed `PopCycle.cpp` and
//! `docs/re/specs/POP/CPopCycle*.md` as reading aids.
//!
//! `m_nPercTypeGroup` is `uint8[12 time][2 week][20 zone type][18 group]`; a row is indexed
//! `((popType & 0x1F) + (week + time * 2) * 20) * 18`. The `PercDataArray`s (`m_nPercOther` ...) are
//! `uint8[12][2][20]`, indexed `zoneType + (week + time * 2) * 20`.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{ftol32, jp5, model_info, rand_at, rand_frac, rd, xi};
use crate::population::{CURRENT_WORLD_ZONE, NUM_PEDS_IN_GROUP, PED_GROUPS, TRANSLATION_ARRAY};

pub const PERC_ROWS: usize = 12 * 2 * 20;
pub const GROUPS: usize = 18;

sa_core::globals! {
    list GLOBALS;
    /// `CPopCycle::m_bCurrentZoneIsGangArea`
    pub static CURRENT_ZONE_IS_GANG_AREA @ 0xC0BC5C: u8 = 0;
    /// `CPopCycle::m_fCurrentZoneDodgyness`
    pub static CURRENT_ZONE_DODGYNESS @ 0xC0BC60: f32 = 0.0;
    /// `CPopCycle::m_pCurrZone` (`CZone*`)
    pub static CURR_ZONE @ 0xC0BC64: GPtr<u8> = GPtr::NULL;
    /// `CPopCycle::m_pCurrZoneInfo` (`CZoneInfo*`)
    pub static CURR_ZONE_INFO @ 0xC0BC68: GPtr<u8> = GPtr::NULL;
    /// `CPopCycle::m_nCurrentZoneType`
    pub static CURRENT_ZONE_TYPE @ 0xC0BC6C: i32 = 0;
    /// `CPopCycle::m_nCurrentTimeOfWeek`
    pub static CURRENT_TIME_OF_WEEK @ 0xC0BC70: i32 = 0;
    /// `CPopCycle::m_nCurrentTimeIndex`
    pub static CURRENT_TIME_INDEX @ 0xC0BC74: i32 = 0;
    /// `CPopCycle::m_nPercTypeGroup`
    pub static PERC_TYPE_GROUP @ 0xC0BC78: [u8; PERC_ROWS * GROUPS] = [0; PERC_ROWS * GROUPS];
    /// `CPopCycle::m_nPercOther`
    pub static PERC_OTHER @ 0xC0DE38: [u8; PERC_ROWS] = [0; PERC_ROWS];
    /// The dealer-strength growth probabilities `CPopCycle::UpdateDealerStrengths` compares against
    /// (`float[15]`, indexed by the clamped strength)
    pub static DEALER_STRENGTH_PROB @ 0x8D24F0: [u32; 15] = [
        0x3D4C_CCCD, 0x3E4C_CCCD, 0x3E99_999A, 0x3EB3_3333, 0x3ECC_CCCD, 0x3F00_0000, 0x3F0C_CCCD,
        0x3F19_999A, 0x3F26_6666, 0x3F33_3333, 0x3F33_3333, 0x3F33_3333, 0x3F33_3333, 0x3F33_3333,
        0x3F33_3333,
    ];
}

/// `CStreaming::ms_NextPedToLoadFromGroup` (sa-stream)
use sa_stream::streaming::{NEXT_PED_TO_LOAD_FROM_GROUP, PEDS_LOADED};

/// The `m_nPercTypeGroup` row of the current time and zone (byte pointer to its first entry).
fn type_group_row() -> GPtr<u8> {
    let week = CURRENT_TIME_OF_WEEK.get();
    let time = CURRENT_TIME_INDEX.get();
    let zi = CURR_ZONE_INFO.get();
    let pop = i32::from(rd::<u8>(zi, 0xF)) & 0x1F;
    let idx = (pop + (week.wrapping_add(time.wrapping_mul(2))).wrapping_mul(20)).wrapping_mul(18);
    PERC_TYPE_GROUP.ptr().cast::<u8>().byte_add(idx as isize)
}

/// `_ftol(((rand() & 0xFFFF) * (1/32768)) * 100.0)`.
fn rand_perc(site: u32) -> i32 {
    ftol32(rand_frac(rand_at(site)).fmul_m(100.0))
}

/// `CPopCycle::IsPedInGroup(int32 modelIndex, int32 popcycleGroup)` (0x610210): whether the model is in
/// one of the three ped groups `m_TranslationArray[group]` names.
#[hook(addr = 0x610210, conv = cdecl, name = "CPopCycle::IsPedInGroup", expect = "8B 44 24 08 53")]
pub fn is_ped_in_group(model: i32, group: i32) -> u8 {
    let tr = TRANSLATION_ARRAY.ptr().cast::<i32>();
    for k in 0..3 {
        let pg = tr.wrapping_index(group.wrapping_mul(3).wrapping_add(k)).read();
        let n = i32::from(NUM_PEDS_IN_GROUP.ptr().cast::<i16>().wrapping_index(pg).read());
        let row = PED_GROUPS.ptr().cast::<u16>().wrapping_index(pg.wrapping_mul(21));
        let mut i = 0;
        while i < n {
            if model == i32::from(row.wrapping_index(i).read()) {
                return 1;
            }
            i += 1;
        }
    }
    0
}

/// `CPopCycle::GetCurrentPercOther_Peds()` (0x610310): `_ftol(m_nPercOther[...] * f)` where
/// `f = 1 - sqrt(CWeather::WetRoads) * 0.8`, replaced by 1.0 while the player is on a mission and
/// drives a model 0x1A4 / 0x1B6 (taxi / cabbie) with the ped's `bInVehicle` set, and while a frenzy runs.
#[hook(addr = 0x610310, conv = cdecl, name = "CPopCycle::GetCurrentPercOther_Peds", expect = "83 EC 08 D9 05 24 13 C8 00")]
pub fn get_current_perc_other_peds() -> i32 {
    let wet = sa_world::world::common::g::<f32>(0xC81324).read();
    let week = CURRENT_TIME_OF_WEEK.get();
    let time = CURRENT_TIME_INDEX.get();
    let zt = CURRENT_ZONE_TYPE.get();
    let idx = zt.wrapping_add(week.wrapping_add(time.wrapping_mul(2)).wrapping_mul(20));
    let perc = i32::from(PERC_OTHER.ptr().cast::<u8>().wrapping_index(idx).read());
    let mut f = X87::from_f32(wet)
        .fsqrt()
        .fmul_m(f32::from_bits(0x3F4C_CCCD))
        .fsubr_m(1.0)
        .to_f32();
    if is_player_on_a_mission() != 0 {
        let ped = crate::common::find_player_ped(-1);
        if !ped.is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
            let veh: GPtr<u8> = rd(ped, 0x58C);
            if !veh.is_null() {
                let m = i32::from(rd::<i16>(veh, 0x22));
                if m == 0x1A4 || m == 0x1B6 {
                    f = 1.0;
                }
            }
        }
    }
    if crate::misc::frenzy_on_going_call() != 0 {
        return ftol32(xi(perc).fmul(X87::from_f32(1.0)));
    }
    ftol32(xi(perc).fmul(X87::from_f32(f)))
}

sa_core::orig! {
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    pub fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    pub fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
}

/// `CPopCycle::PickARandomGroupOfOtherPeds()` (0x610420): a random percentage walked down the current
/// row while `p[g] < r` (`r -= p[g]`).
#[hook(addr = 0x610420, conv = cdecl, name = "CPopCycle::PickARandomGroupOfOtherPeds", expect = "51 53 56 E8 F6 16 21 00")]
pub fn pick_a_random_group_of_other_peds() -> i32 {
    let mut r = rand_perc(0x610428);
    let p = type_group_row();
    let mut g = 0;
    loop {
        let v = i32::from(p.wrapping_index(g).read());
        if v >= r {
            return g;
        }
        r -= v;
        g += 1;
    }
}

/// `CPopCycle::UpdateDealerStrengths()` (0x6104B0): while gang wars are active, once per minute
/// period of `CTimer` (`time / 60000` changed since the previous frame), every zone info with one of the
/// first three gang strengths above 10 grows its dealer strength (+0xA, up to 15) when
/// `rand() * 3.0518509e-05 < prob[min(strength, 14)]`.
#[hook(addr = 0x6104B0, conv = cdecl, name = "CPopCycle::UpdateDealerStrengths", expect = "51 A0 90 AB 96 00")]
pub fn update_dealer_strengths() {
    if crate::gangs::GANG_WARS_ACTIVE.get() == 0 {
        return;
    }
    let prev = sa_core::timer::PREV_TIME_MS.get() / 60000;
    let now = sa_core::timer::TIME_MS.get() / 60000;
    if now == prev {
        return;
    }
    let infos = sa_world::world::zones::ZONE_INFOS.ptr().cast::<u8>();
    let mut i = 0u32;
    while i < u32::from(sa_world::world::zones::TOTAL_ZONE_INFOS.get()) {
        let zi = infos.byte_add((i * 0x11) as isize);
        if rd::<u8>(zi, 1) > 10 || rd::<u8>(zi, 0) > 10 || rd::<u8>(zi, 2) > 10 {
            let s = rd::<u8>(zi, 0xA);
            let k = if s >= 14 { 14 } else { i32::from(s) };
            let r = rand_at(0x610518);
            let prob = f32::from_bits(DEALER_STRENGTH_PROB.ptr().cast::<u32>().wrapping_index(k).read());
            let v = xi(r).fmul_m(f32::from_bits(0x3800_0100));
            if !jp5(v, X87::from_f32(prob)) {
                let s = rd::<u8>(zi, 0xA);
                if s < 15 {
                    crate::common::wr(zi, 0xA, s + 1);
                }
            }
        }
        i += 1;
    }
}

/// `CPopCycle::UpdateAreaDodgyness()` (0x610560): `dealer * 0.07 + sum(gang[0..10] * 0.01)`, accumulated
/// through the f32 global as the original does, capped at 1.0.
#[hook(addr = 0x610560, conv = cdecl, name = "CPopCycle::UpdateAreaDodgyness", expect = "51 A1 68 BC C0 00")]
pub fn update_area_dodgyness() {
    let zi = CURR_ZONE_INFO.get();
    CURRENT_ZONE_DODGYNESS.set(0.0);
    CURRENT_ZONE_DODGYNESS.set(
        xi(i32::from(rd::<u8>(zi, 0xA)))
            .fmul_m(f32::from_bits(0x3D8F_5C29))
            .to_f32(),
    );
    let mut last = X87::from_f32(0.0);
    for k in 0..10 {
        let v = xi(i32::from(rd::<u8>(zi, k)))
            .fmul_m(f32::from_bits(0x3C23_D70A))
            .fadd_m(CURRENT_ZONE_DODGYNESS.get());
        CURRENT_ZONE_DODGYNESS.set(v.to_f32());
        last = v;
    }
    // fst; fcomp 1.0; test ah, 5; jnp -> skip (ST < 1.0)
    if jp5(last, X87::from_f32(1.0)) {
        CURRENT_ZONE_DODGYNESS.set(1.0);
    }
}

/// `CPopCycle::PedIsAcceptableInCurrentZone(int32 modelIndex)` (0x610720).
#[hook(addr = 0x610720, conv = cdecl, name = "CPopCycle::PedIsAcceptableInCurrentZone", expect = "56 8B 35 68 BC C0 00")]
pub fn ped_is_acceptable_in_current_zone(model: i32) -> u8 {
    let zi = CURR_ZONE_INFO.get();
    if zi.is_null() {
        return 0;
    }
    if is_zone_streaming_allowed() != 0 {
        return 1;
    }
    let race = u32::from(rd::<u8>(model_info(model), 0x3A));
    if race == 0 {
        return 1;
    }
    let bit = 1u32.wrapping_shl(race - 1) as u8;
    if (rd::<u8>(zi, 0x10) & 0xF) & bit != 0 {
        1
    } else {
        0
    }
}

sa_core::orig! {
    /// `CCheat::IsZoneStreamingAllowed()` (0x407410)
    pub fn is_zone_streaming_allowed() -> u8 = cdecl 0x407410 => sa_world::world::cheat::is_zone_streaming_allowed;
}

/// `CPopCycle::PickPedMIToStreamInForCurrentZone()` (0x60FFD0): see `docs/re/specs/POP/CPopCycle.md`.
/// The group pick stops at `r < p[g]` (not `PickARandomGroupOfOtherPeds`' `p[g] >= r`).
#[hook(addr = 0x60FFD0, conv = cdecl, name = "CPopCycle::PickPedMIToStreamInForCurrentZone", expect = "83 EC 08 53 55")]
pub fn pick_ped_mi_to_stream_in_for_current_zone() -> i32 {
    let next = NEXT_PED_TO_LOAD_FROM_GROUP.ptr().cast::<i32>();
    let loaded = PEDS_LOADED.ptr().cast::<i32>();
    for _ in 0..10 {
        let mut r = rand_perc(0x60FFE5);
        let p = type_group_row();
        let mut g = 0i32;
        loop {
            let v = i32::from(p.wrapping_index(g).read());
            if r < v {
                break;
            }
            r -= v;
            g += 1;
        }
        let zone = CURRENT_WORLD_ZONE.get() as i32;
        let pg = TRANSLATION_ARRAY
            .ptr()
            .cast::<i32>()
            .wrapping_index(g.wrapping_mul(3).wrapping_add(zone))
            .read();
        let n = i32::from(NUM_PEDS_IN_GROUP.ptr().cast::<i16>().wrapping_index(pg).read());
        let mut k = 0;
        while k < n {
            let slot = next.wrapping_index(g).read().wrapping_add(1) % n;
            let mi = i32::from(
                PED_GROUPS
                    .ptr()
                    .cast::<u16>()
                    .wrapping_index(pg.wrapping_mul(21).wrapping_add(slot))
                    .read(),
            );
            next.wrapping_index(g).write(slot);
            let mut ok = true;
            for i in 0..8 {
                if loaded.wrapping_index(i).read() == mi {
                    ok = false;
                }
            }
            let race = u32::from(rd::<u8>(model_info(mi), 0x3A));
            if race != 0 {
                let bit = 1u32.wrapping_shl(race - 1) as u8;
                let zr = rd::<u8>(CURR_ZONE_INFO.get(), 0x10) & 0xF;
                if zr & bit == 0 {
                    ok = false;
                }
            }
            k += 1;
            if ok {
                return mi;
            }
        }
    }
    -1
}

sa_core::orig! {
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250)
    pub fn find_player_centre_of_world(id: i32) -> GPtr<u8> = cdecl 0x56E250 => sa_world::world::cworld::find_player_centre_of_world;
    /// `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400)
    pub fn get_zone_info(p: GPtr<f32>, out: GPtr<GPtr<u8>>) -> GPtr<u8> = cdecl 0x572400 => sa_world::world::zones::get_zone_info;
    /// `CPopCycle::UpdatePercentages()` (0x610770)
    pub fn update_percentages_call() = cdecl 0x610770 => crate::population2::update_percentages;
    /// `CPopCycle::UpdateDealerStrengths()` (0x6104B0)
    pub fn update_dealer_strengths_call() = cdecl 0x6104B0 => update_dealer_strengths;
    /// `CPopCycle::UpdateAreaDodgyness()` (0x610560)
    pub fn update_area_dodgyness_call() = cdecl 0x610560 => update_area_dodgyness;
}

/// `CClock::CurrentDay` (0x0B7014E; 1 = Sunday)
const CLOCK_CURRENT_DAY: usize = 0xB7014E;

/// `CPopCycle::Update()` (0x610BF0): time of week (jump table 0x610D0C: days 0/7 weekend, Sunday
/// weekend until 20h, Friday... day 6 weekend from 20h), time index `hour / 2`, the zone info of the
/// player's centre of world (kept while z >= 950 once set), the percentages, dealer strengths,
/// dodgyness, and the gang-area flag (gang strengths summed > 20).
#[hook(addr = 0x610BF0, conv = cdecl, name = "CPopCycle::Update", expect = "0F B6 05 4E 01 B7 00")]
pub fn update() {
    let day = crate::common::g::<u8>(CLOCK_CURRENT_DAY).read();
    let hour = sa_core::light::CLOCK_HOURS.get();
    match day {
        0 | 7 => CURRENT_TIME_OF_WEEK.set(1),
        1 => CURRENT_TIME_OF_WEEK.set(i32::from(hour < 20)),
        2..=5 => CURRENT_TIME_OF_WEEK.set(0),
        6 => CURRENT_TIME_OF_WEEK.set(i32::from(hour >= 20)),
        _ => {}
    }
    CURRENT_TIME_INDEX.set(i32::from(hour >> 1));
    let c = find_player_centre_of_world(i32::from(sa_world::world::cworld::PLAYER_IN_FOCUS.get()));
    let mut p = [
        c.cast::<u32>().read(),
        c.cast::<u32>().wrapping_index(1).read(),
        c.cast::<u32>().wrapping_index(2).read(),
    ];
    let high = jp5(X87::from_f32(f32::from_bits(p[2])), X87::from_f32(950.0));
    if !high || CURR_ZONE_INFO.get().is_null() {
        let zi = get_zone_info(GPtr::from_raw(p.as_mut_ptr()).cast(), CURR_ZONE.ptr());
        CURR_ZONE_INFO.set(zi);
        CURRENT_ZONE_TYPE.set(i32::from(rd::<u8>(zi, 0xF)) & 0x1F);
    }
    update_percentages_call();
    update_dealer_strengths_call();
    update_area_dodgyness_call();
    let zi = CURR_ZONE_INFO.get();
    let mut sum = 0i32;
    for k in 0..10 {
        sum += i32::from(rd::<u8>(zi, k));
    }
    CURRENT_ZONE_IS_GANG_AREA.set(u8::from(sum > 20));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE,
    HOOK_IS_PED_IN_GROUP,
    HOOK_GET_CURRENT_PERC_OTHER_PEDS,
    HOOK_PICK_A_RANDOM_GROUP_OF_OTHER_PEDS,
    HOOK_UPDATE_DEALER_STRENGTHS,
    HOOK_UPDATE_AREA_DODGYNESS,
    HOOK_PED_IS_ACCEPTABLE_IN_CURRENT_ZONE,
    HOOK_PICK_PED_MI_TO_STREAM_IN_FOR_CURRENT_ZONE,
];
