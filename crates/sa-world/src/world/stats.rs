//! `CStats` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed `Stats.cpp` as reading aid).
//!
//! Stat ids below 0x52 are floats (`StatTypesFloat`, 0xB79380), ids from 0x78 are ints (`StatTypesInt`,
//! 0xB79000, indexed `id - 0x78`; the code addresses it as `0xB78E20 + id*4`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use super::common::{ftol32, g, x, xi};
use super::cworld::{PLAYERS, PLAYER_INFO_SIZE, PLAYER_IN_FOCUS};

sa_core::globals! {
    list GLOBALS;
    /// `CStats::TimesMissionAttempted` (`int32[100]`)
    pub static TIMES_MISSION_ATTEMPTED @ 0xB78CC8: [i32; 100] = [0; 100];
    /// `CStats::FavoriteRadioStationList` (`int32[14]`)
    pub static FAVORITE_RADIO_STATIONS @ 0xB78E58: [i32; 14] = [0; 14];
    /// `CStats::StatTypesInt` (`int32[223]`, ids 0x78..)
    pub static STAT_TYPES_INT @ 0xB79000: [i32; 223] = [0; 223];
    /// `CStats::StatTypesFloat` (`float[82]`, ids 0..0x51)
    pub static STAT_TYPES_FLOAT @ 0xB79380: [f32; 82] = [0.0; 82];
}

/// `gString` (0xB71670), the shared text buffer `GetStatID` prints into.
const G_STRING: usize = 0xB71670;
/// the byte `FindCriminalRatingNumber` tests at 0x96918C (gr: `CCheat::...`-independent flag; unnamed)
const RATING_FLAG_96918C: usize = 0x96918C;

orig! {
    /// CRT `sprintf` (0x821BB5) with one int argument
    pub fn sprintf_i(buf: GPtr<u8>, fmt: GPtr<u8>, v: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CLocalisation::GermanGame` (0x56D200)
    pub fn german_game() -> u8 = cdecl 0x56D200 => unported;
    /// `CStats::CheckForStatsMessage` (0x559760)
    pub fn check_for_stats_message() = cdecl 0x559760 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    pub fn increment_stat_call(id: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CStats::DecrementStat(uint16, float)` (0x559FA0)
    pub fn decrement_stat_call(id: u32, v: u32) = cdecl 0x559FA0 => decrement_stat_port;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32, float)` (0x55B980)
    pub fn display_script_stat_update_message(state: u32, stat: u32, v: u32) = cdecl 0x55B980 => unported;
    /// `CStats::UpdateFatAndMuscleStats(uint32)` (0x55C470)
    pub fn update_fat_and_muscle_stats(v: i32) = cdecl 0x55C470 => update_fat_and_muscle_stats_host;
}

fn int_stat(id: u32) -> i32 {
    STAT_TYPES_INT.ptr().cast::<i32>().wrapping_index(id as i32 - 0x78).read()
}

fn float_stat(id: u32) -> f32 {
    STAT_TYPES_FLOAT.ptr().cast::<f32>().wrapping_index(id as i32).read()
}

/// `CStats::GetStatID(uint16)` (0x558DE0): `sprintf(gString, id < 0x52 ? "stat_f_%d" : "stat_i_%d", id)`;
/// returns `gString`.
#[hook(addr = 0x558DE0, conv = cdecl, name = "CStats::GetStatID", expect = "66 8B 44 24 04")]
pub fn get_stat_id(id: u16) -> GPtr<u8> {
    let fmt = if id < 0x52 { 0x863EA8 } else { 0x863E9C };
    sprintf_i(g(G_STRING), g(fmt), i32::from(id));
    g(G_STRING)
}

/// `CStats::IsStatFloat(uint16)` (0x558E30): `id < 0x52`.
#[hook(addr = 0x558E30, conv = cdecl, name = "CStats::IsStatFloat", expect = "66 83 7C 24 04 52")]
pub fn is_stat_float(id: u16) -> u8 {
    (id < 0x52) as u8
}

/// `CStats::GetStatValue(uint16)` (0x558E40): `fld` of the float stat or `fild` of the int stat, in ST0.
#[hook(addr = 0x558E40, conv = cdecl, name = "CStats::GetStatValue--OG", expect = "66 8B 44 24 04")]
pub fn get_stat_value(id: u16) -> X87 {
    if id < 0x52 { x(float_stat(u32::from(id))) } else { xi(int_stat(u32::from(id))) }
}

/// `CStats::GetTimesMissionAttempted(uint8)` (0x558E70): the low byte of the count (AL).
#[hook(addr = 0x558E70, conv = cdecl, name = "CStats::GetTimesMissionAttempted", expect = "0F B6 44 24 04")]
pub fn get_times_mission_attempted(m: u8) -> u8 {
    TIMES_MISSION_ATTEMPTED.ptr().cast::<u8>().byte_add(4 * isize::from(m)).read()
}

/// `CStats::RegisterMissionAttempted(uint8)` (0x558E80): `++` unless the count is -1 (passed).
#[hook(addr = 0x558E80, conv = cdecl, name = "CStats::RegisterMissionAttempted", expect = "0F B6 44 24 04")]
pub fn register_mission_attempted(m: u8) {
    let p = TIMES_MISSION_ATTEMPTED.ptr().cast::<i32>().wrapping_index(i32::from(m));
    let c = p.read();
    if c != -1 {
        p.write(c.wrapping_add(1));
    }
}

/// `CStats::RegisterMissionPassed(uint8)` (0x558EA0): count = -1.
#[hook(addr = 0x558EA0, conv = cdecl, name = "CStats::RegisterMissionPassed", expect = "0F B6 44 24 04")]
pub fn register_mission_passed(m: u8) {
    TIMES_MISSION_ATTEMPTED.ptr().cast::<i32>().wrapping_index(i32::from(m)).write(-1);
}

/// `CStats::GetFullFavoriteRadioStationList` (0x558F90).
#[hook(addr = 0x558F90, conv = cdecl, name = "CStats::GetFullFavoriteRadioStationList", expect = "B8 58 8E B7 00")]
pub fn get_full_favorite_radio_station_list() -> GPtr<i32> {
    FAVORITE_RADIO_STATIONS.ptr().cast()
}

/// `CStats::FindMostFavoriteRadioStation` (0x558FA0): over stations 1..12, the first with the largest
/// count above 0 (signed, strict); 1 when none.
#[hook(addr = 0x558FA0, conv = cdecl, name = "CStats::FindMostFavoriteRadioStation", expect = "56 57 33 C9 B8 01 00 00 00")]
pub fn find_most_favorite_radio_station() -> i32 {
    let l = FAVORITE_RADIO_STATIONS.get();
    let (mut best, mut r) = (0i32, 1i32);
    for (i, &v) in l.iter().enumerate().take(13).skip(1) {
        if v > best {
            best = v;
            r = i as i32;
        }
    }
    r
}

/// `CStats::FindLeastFavoriteRadioStation` (0x559010): over stations 1..12, the first with the smallest
/// count (starting from station 1's).
#[hook(addr = 0x559010, conv = cdecl, name = "CStats::FindLeastFavoriteRadioStation", expect = "8B 0D 5C 8E B7 00")]
pub fn find_least_favorite_radio_station() -> i32 {
    let l = FAVORITE_RADIO_STATIONS.get();
    let (mut best, mut r) = (l[1], 1i32);
    for (i, &v) in l.iter().enumerate().take(13).skip(1) {
        if v < best {
            best = v;
            r = i as i32;
        }
    }
    r
}

fn si(off: usize) -> i32 {
    STAT_TYPES_INT.ptr().cast::<u8>().byte_add(off as isize).cast::<i32>().read()
}

/// `CStats::FindCriminalRatingNumber` (0x559080): the x87 sequence of the bytes (see the docs), with
/// `_ftol` truncations at the same points.
#[hook(addr = 0x559080, conv = cdecl, name = "CStats::FindCriminalRatingNumber", expect = "51 DB 05 E4 90 B7 00")]
pub fn find_criminal_rating_number() -> i32 {
    let money = g::<i32>(PLAYERS + usize::from(PLAYER_IN_FOCUS.get()) * PLAYER_INFO_SIZE + 0xB8).read();
    let m5000 = money / 5000;
    let mut st = xi(si(0xE4)).fsub(xi(si(0x34)).fmul_m(3.0));
    st = st.fsub(xi(si(0x3C)).fmul_m(3.0));
    st = st.fadd(xi(si(0x9C)).fadd(xi(si(0x98))).fmul_m(10.0));
    st = st.fadd(xi(m5000));
    st = st.fadd(xi(si(0x10)).fmul_m(30.0));
    st = st.fadd(xi(si(0x84)));
    st = st.fadd(xi(si(0x80)));
    st = st.fadd(xi(si(0x7C)));
    let mut r = ftol32(st);
    let penalty = g::<u8>(RATING_FLAG_96918C).read() != 0
        || !xi(si(0x44)).fcom(X87::zero()).test_any(0x41);
    if penalty {
        r = ftol32(xi(r).fsub(xi(si(0x44)).fmul_m(10.0)));
        if r <= -10000 {
            r = -10000;
        }
    } else if r <= 0 {
        r = 0;
    }
    let i18 = xi(si(0x18));
    // fcom 100.0; test ah,0x41; jne: only I18 > 100
    if !i18.fcom(x(100.0)).test_any(0x41) {
        r = ftol32(xi(si(0x20)).fdiv(i18).fmul_m(500.0).fadd(xi(r)));
    }
    let total = float_stat(0x01);
    // test ah,0x44; jnp: return r when total == 0
    if x(total).fcom(X87::zero()).test_parity(0x44) {
        return ftol32(x(float_stat(0x00)).fdiv_m(total).fmul_m(1000.0).fadd(xi(r)));
    }
    r
}

/// `CStats::GetPercentageProgress` (0x5591E0): 0 when the total (float stat 1) == 0, else
/// `progress / total * 100`, capped at 100 when above; ST0.
#[hook(addr = 0x5591E0, conv = cdecl, name = "CStats::GetPercentageProgress", expect = "D9 05 84 93 B7 00")]
pub fn get_percentage_progress() -> X87 {
    let total = float_stat(0x01);
    if !x(total).fcom(X87::zero()).test_parity(0x44) {
        return X87::zero();
    }
    let v = x(float_stat(0x00)).fdiv_m(total).fmul_m(100.0);
    // fld 100; fcomp st(1); test ah,5; jp: keep v unless 100 < v
    if x(100.0).fcom(v).test_parity(0x05) { v } else { x(100.0) }
}

/// `CStats::ConvertToMins(int32)` (0x559540): 0 below 60, else `(uint32)(v - 60) / 60 + 1`.
#[hook(addr = 0x559540, conv = cdecl, name = "CStats::ConvertToMins", expect = "8B 4C 24 04 33 C0")]
pub fn convert_to_mins(v: i32) -> i32 {
    if v <= 0x3B { 0 } else { ((v.wrapping_sub(60) as u32) / 60 + 1) as i32 }
}

/// `CStats::ConvertToSecs(int32)` (0x559560): `v` below 60, else `(v - 60) - (uint32)(v - 60) / 60 * 60`;
/// then the absolute value (`neg`).
#[hook(addr = 0x559560, conv = cdecl, name = "CStats::ConvertToSecs", expect = "8B 4C 24 04 83 F9 3B")]
pub fn convert_to_secs(v: i32) -> i32 {
    let mut c = v;
    if c > 0x3B {
        let q = (c.wrapping_sub(60) as u32) / 60;
        c = c.wrapping_add((-60i32).wrapping_sub(q.wrapping_mul(60) as i32));
    }
    if c < 0 { c.wrapping_neg() } else { c }
}

/// `CStats::SafeToShowThisStat(uint8)` (0x559590): true unless a German game and the id is one of
/// 0xA7, 0xA8, 0xB1, 0xCD..0xD1 (the byte table at 0x5595C4).
#[hook(addr = 0x559590, conv = cdecl, name = "CStats::SafeToShowThisStat", expect = "E8 6B 3C 01 00")]
pub fn safe_to_show_this_stat(id: u8) -> u8 {
    const TABLE: [u8; 43] = [
        0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        0, 0, 0, 0, 0,
    ];
    if german_game() == 0 {
        return 1;
    }
    let i = u32::from(id).wrapping_sub(0xA7);
    if i > 0x2A {
        return 1;
    }
    TABLE[i as usize]
}

/// `CStats::CheckForThreshold(float* stat, float threshold)` (0x5595F0): when `*stat + 40 < t` or
/// `*stat - 40 > t`, `*stat = t` (bit copy) and true.
#[hook(addr = 0x5595F0, conv = cdecl, name = "CStats::CheckForThreshold", expect = "8B 4C 24 04 D9 01")]
pub fn check_for_threshold(stat: GPtr<u32>, t: u32) -> u8 {
    let s = f32::from_bits(stat.read());
    let tf = x(f32::from_bits(t));
    let set = !x(s).fadd_m(40.0).fcom(tf).test_parity(0x05) || !x(s).fsub_m(40.0).fcom(tf).test_any(0x41);
    if set {
        stat.write(t);
        return 1;
    }
    0
}

/// `CStats::IsStatCapped(uint16)` (0x559630).
#[hook(addr = 0x559630, conv = cdecl, name = "CStats::IsStatCapped", preserve = "ecx", expect = "66 8B 44 24 04")]
pub fn is_stat_capped(id: u16) -> u8 {
    matches!(
        id,
        0x41 | 0x42
            | 0x43
            | 0x15..=0x19
            | 0x45..=0x50
            | 0xA4
            | 0xA5
            | 0xA0
            | 0xDF
            | 0xE1
            | 0xE5
            | 0xE9
            | 0xF1..=0xF4
            | 0xE6
    ) as u8
}

pub fn decrement_stat_port(id: u32, v: u32) {
    decrement_stat(id as u16, v)
}

fn set_int_stat(id: u32, v: i32) {
    STAT_TYPES_INT.ptr().cast::<i32>().wrapping_index(id as i32 - 0x78).write(v)
}

fn set_float_stat(id: u32, v: f32) {
    STAT_TYPES_FLOAT.ptr().cast::<f32>().wrapping_index(id as i32).write(v)
}

/// `CStats::StatReactionValue[k]` (0xB78F10)
fn reaction(k: usize) -> f32 {
    g::<f32>(0xB78F10 + 4 * k).read()
}

/// `fild` of a uint32 counter: + 2^32 when the sign bit is set (0x858C54).
fn u32_as_x87(c: u32) -> X87 {
    let v = xi(c as i32);
    if (c as i32) < 0 { v.fadd_m(4_294_967_296.0) } else { v }
}

/// `CStats::SetStatValue(uint16, float)` (0x55A070): float stats take the value (`fld`/`fstp`), int
/// stats `(int16)_ftol(v)` sign-extended (original); then `CheckForStatsMessage` (tail jump).
#[hook(addr = 0x55A070, conv = cdecl, name = "CStats::SetStatValue", expect = "D9 44 24 08 56")]
pub fn set_stat_value(id: u16, v: f32) {
    let id = u32::from(id);
    if id < 0x52 {
        set_float_stat(id, x(v).to_f32());
    } else {
        set_int_stat(id, ftol32(x(v)) as i16 as i32);
    }
    check_for_stats_message();
}

/// `CStats::ModifyStat(uint16, float)` (0x55D090): `IncrementStat(id, v)` (tail jump with the caller's
/// arguments) unless `v < 0` or NaN, then `DecrementStat(id, -v)`.
#[hook(addr = 0x55D090, conv = cdecl, name = "CStats::ModifyStat", expect = "D9 44 24 08 D8 1D 50 8B 85 00")]
pub fn modify_stat(id: u32, v: u32) {
    if !x(f32::from_bits(v)).fcom(X87::zero()).test_any(0x01) {
        increment_stat_call(id, v);
        return;
    }
    decrement_stat_call(id, x(f32::from_bits(v)).fchs().to_f32().to_bits());
}

/// `CStats::ProcessReactionStatsOnDecrement(uint8)` (0x559730): for stat 0xA5 below 0, fat (0x15) -= 23.
#[hook(addr = 0x559730, conv = cdecl, name = "CStats::ProcessReactionStatsOnDecrement", expect = "80 7C 24 04 A5")]
pub fn process_reaction_stats_on_decrement(id: u8) {
    if id == 0xA5 && !xi(int_stat(0xA5)).fcom(X87::zero()).test_parity(0x05) {
        decrement_stat_call(0x15, 0x41B8_0000);
    }
}

/// `CStats::ProcessReactionStatsOnIncrement(uint8)` (0x55B900): stamina (0x16) / 0xE1 with stat 0xA5
/// below 0: fat = max(fat - 23, 0) and `CheckForStatsMessage`; stat 0xA5 above 1000: fat += stat - 1000.
#[hook(addr = 0x55B900, conv = cdecl, name = "CStats::ProcessReactionStatsOnIncrement", expect = "0F B6 44 24 04")]
pub fn process_reaction_stats_on_increment(id: u8) {
    match id {
        0x16 | 0xE1 => {
            if xi(int_stat(0xA5)).fcom(X87::zero()).test_parity(0x05) {
                return;
            }
            let f = x(float_stat(0x15)).fsub_m(23.0);
            let v = if !f.fcom(X87::zero()).test_any(0x41) { f } else { X87::zero() };
            set_float_stat(0x15, v.to_f32());
            check_for_stats_message();
        }
        0xA5 => {
            let i = xi(int_stat(0xA5));
            if !i.fcom(x(1000.0)).test_any(0x41) {
                increment_stat_call(0x15, i.fsub_m(1000.0).to_f32().to_bits());
            }
        }
        _ => {}
    }
}

/// `CStats::DecrementStat(uint16, float)` (0x559FA0): only for `v > 0`. Float stats: `max(s - v, 0)`;
/// int stats: `s - (int16)_ftol(v)`, then 0 when not above 0; low byte 0xA5 below 0 → fat -= 23; then
/// `CheckForStatsMessage` (tail jump).
#[hook(addr = 0x559FA0, conv = cdecl, name = "CStats::DecrementStat", expect = "D9 44 24 08 D8 1D 50 8B 85 00")]
pub fn decrement_stat(id: u16, v: u32) {
    let vf = f32::from_bits(v);
    if x(vf).fcom(X87::zero()).test_any(0x41) {
        return;
    }
    let idw = u32::from(id);
    if idw < 0x52 {
        let r = x(float_stat(idw)).fsub_m(vf);
        set_float_stat(idw, r.to_f32());
        let fin = if !r.fcom(X87::zero()).test_any(0x41) { r } else { X87::zero() };
        set_float_stat(idw, fin.to_f32());
    } else {
        let e = ftol32(x(vf)) as i16 as i32;
        let n = int_stat(idw).wrapping_sub(e);
        set_int_stat(idw, n);
        let f = xi(n);
        let fin = if !f.fcom(X87::zero()).test_any(0x41) { f } else { X87::zero() };
        set_int_stat(idw, ftol32(fin));
    }
    if id as u8 == 0xA5 && !xi(int_stat(0xA5)).fcom(X87::zero()).test_parity(0x05) {
        decrement_stat_call(0x15, 0x41B8_0000);
    }
    check_for_stats_message();
}

/// The shared body of the sprint/run counters (`StatReactionValue` indices, counter, threshold, stat
/// amount cell).
fn update_counter(reaction_fat: usize, counter: usize, threshold: usize, amount: usize) {
    update_fat_and_muscle_stats(ftol32(x(reaction(reaction_fat))));
    let c = g::<u32>(counter).read();
    let cf = u32_as_x87(c);
    let lim = x(reaction(threshold)).fmul_m(1000.0);
    // fcompp (st0 = lim, st1 = counter); test ah,5; jp: the reward when lim < counter
    if !lim.fcom(cf).test_parity(0x05) {
        g::<u32>(counter).write(0);
        let a = g::<u32>(amount).read();
        increment_stat_call(0x16, a);
        display_script_stat_update_message(1, 0x16, g::<u32>(amount).read());
        return;
    }
    let step = g::<f32>(0xB7CB5C).read();
    let add = ftol32(x(step).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0));
    g::<u32>(counter).write(g::<u32>(counter).read().wrapping_add(add as u32));
}

/// `CStats::UpdateStatsWhenSprinting` (0x55C660): fat/muscle update with reaction 30, then the sprint
/// stamina counter (0xB794D8) against reaction 35 * 1000 (stamina += reaction 2 and the message), else
/// counter += `_ftol(timeStep * 0.02 * 1000)`.
#[hook(addr = 0x55C660, conv = cdecl, name = "CStats::UpdateStatsWhenSprinting", expect = "51 D9 05 88 8F B7 00")]
pub fn update_stats_when_sprinting() {
    update_counter(30, 0xB794D8, 35, 0xB78F18);
}

/// `CStats::UpdateStatsWhenRunning` (0x55C6F0): as sprinting with reaction 31, counter 0xB79500,
/// threshold reaction 36, amount reaction 3.
#[hook(addr = 0x55C6F0, conv = cdecl, name = "CStats::UpdateStatsWhenRunning", expect = "51 D9 05 8C 8F B7 00")]
pub fn update_stats_when_running() {
    update_counter(31, 0xB79500, 36, 0xB78F1C);
}

/// `CStats::UpdateStatsWhenFighting` (0x55CFA0): `UpdateFatAndMuscleStats(_ftol(reaction 32))`.
#[hook(addr = 0x55CFA0, conv = cdecl, name = "CStats::UpdateStatsWhenFighting", expect = "D9 05 90 8F B7 00")]
pub fn update_stats_when_fighting() {
    update_fat_and_muscle_stats(ftol32(x(reaction(32))));
}

/// `CStats::UpdateStatsAddToHealth(uint32)` (0x55D030): counter 0xB79510 += amount; above reaction 58:
/// stat 0x18 += reaction 11, the message, counter 0.
#[hook(addr = 0x55D030, conv = cdecl, name = "CStats::UpdateStatsAddToHealth", expect = "A1 10 95 B7 00")]
pub fn update_stats_add_to_health(add: u32) {
    let c = g::<u32>(0xB79510).read().wrapping_add(add);
    g::<u32>(0xB79510).write(c);
    if !u32_as_x87(c).fcom(x(reaction(58))).test_any(0x41) {
        increment_stat_call(0x18, g::<u32>(0xB78F3C).read());
        display_script_stat_update_message(1, 0x18, g::<u32>(0xB78F3C).read());
        g::<u32>(0xB79510).write(0);
    }
}

/// `CStats::SetNewRecordStat(uint16, float)` (0x55C410 → 0x408235 → 0x55C415): `max(stat, v)` (NaN → v);
/// int stats stored back as `(int16)_ftol`, even when unchanged (original); `CheckForStatsMessage`.
// Not hooked: the protection code at 0x4017AD reads the entry dword 0x55C410 as data (`sbb eax, [0x55C410]`),
// so patching the entry breaks it (L2 crash, world bisect h0). Ported, unhookable.
#[sa_core::native(addr = 0x55C410, conv = cdecl, name = "CStats::SetNewRecordStat")]
pub fn set_new_record_stat(id: u16, v: f32) {
    let idw = u32::from(id);
    let cur = if idw < 0x52 { x(float_stat(idw)) } else { xi(int_stat(idw)) };
    let r = if !cur.fcom(x(v)).test_any(0x41) { cur } else { x(v) };
    if idw < 0x52 {
        set_float_stat(idw, r.to_f32());
    } else {
        set_int_stat(idw, ftol32(r) as i16 as i32);
    }
    check_for_stats_message();
}

/// The accumulators of `UpdateFatAndMuscleStats` (u32): fat-burn units (0xB794FC), muscle units
/// (0xB7950C); the message toggle byte (0xB79534).
const FAT_UNITS: usize = 0xB794FC;
const MUSCLE_UNITS: usize = 0xB7950C;
const FAT_MESSAGE_TOGGLE: usize = 0xB79534;
/// The fat stat (float stat 0x15) and the muscle stat (float stat 0x17).
const STAT_FAT: usize = 0xB793D4;
const STAT_MUSCLE: usize = 0xB793DC;

fn gf(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `fild dword` of an accumulator, plus 2^32 when negative (the unsigned conversion).
fn units(addr: usize) -> X87 {
    let v = g::<i32>(addr).read();
    let f = xi(v);
    if v < 0 { f.fadd_m(4_294_967_296.0) } else { f }
}

/// `_ftol(CTimer::ms_fTimeStep * 0.02 * 1000.0)` (x87).
fn step_ms() -> i32 {
    ftol32(x(gf(0xB7CB5C)).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0))
}

/// `stat = max(stat - [rate], 0)` when `[rate]` > 0 (x87 `fsub`, `fcom 0.0`; a NaN difference gives 0),
/// then `CheckForStatsMessage()`.
fn decrease(stat: usize, rate: usize) {
    if !matches!(x(gf(rate)).fcom(X87::zero()), Cmp::Greater) {
        return;
    }
    let t = x(gf(stat)).fsub_m(gf(rate));
    let v = if matches!(t.fcom(X87::zero()), Cmp::Greater) { t.to_f32() } else { 0.0 };
    g::<f32>(stat).write(v);
    check_for_stats_message();
}

/// Phase B/C path of [`update_fat_and_muscle_stats`].
pub fn update_fat_and_muscle_stats_host(v: i32) {
    update_fat_and_muscle_stats_port(v as u32)
}

/// `CStats::UpdateFatAndMuscleStats(uint32 fatBurned)` (0x55C470, cdecl). Fat: when 1000 * `[0xB78FB8]`
/// is below the fat units (0xB794FC, unsigned; x87): the units become 0; with fat (0xB793D4) above 0
/// it drops by `[0xB78F6C]` (when that is above 0; floored at 0, `CheckForStatsMessage`),
/// `IncrementStat(0x17, [0xB78F34])` and the message toggle (0xB79534): 1 → 2 and
/// `DisplayScriptStatUpdateMessage(1, 0x17, [0xB78F34])`, else → 1 and `(0, 0x15, [0xB78F6C])`; without
/// fat the muscle (0xB793DC) drops by `[0xB78F70]` the same way, the toggle becomes 3 and
/// `(0, 0x17, [0xB78F70])`. Otherwise the units grow by `_ftol(timeStep * 0.02 * 1000) * fatBurned / 10`
/// (32-bit unsigned). Muscle: when 1000 * `[0xB78FC4]` is below the muscle units (0xB7950C):
/// `IncrementStat(0x18, [0xB78F3C])`, `DisplayScriptStatUpdateMessage(1, 0x18, [0xB78F3C])`, units 0;
/// else the units grow by `_ftol(timeStep * 0.02 * 1000)`.
#[hook(addr = 0x55c470, conv = cdecl, name = "CStats::UpdateFatAndMuscleStats", expect = "51 A1 FC 94 B7 00",
       unverified = "mvpi-world-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn update_fat_and_muscle_stats_port(fat_burned: u32) {
    let thr = x(gf(0xB78FB8)).fmul_m(1000.0);
    if matches!(thr.fcom(units(FAT_UNITS)), Cmp::Less) {
        let fat = gf(STAT_FAT);
        g::<u32>(FAT_UNITS).write(0);
        if matches!(x(fat).fcom(X87::zero()), Cmp::Greater) {
            decrease(STAT_FAT, 0xB78F6C);
            increment_stat_call(0x17, g::<u32>(0xB78F34).read());
            if g::<u8>(FAT_MESSAGE_TOGGLE).read() == 1 {
                g::<u8>(FAT_MESSAGE_TOGGLE).write(2);
                display_script_stat_update_message(1, 0x17, g::<u32>(0xB78F34).read());
            } else {
                g::<u8>(FAT_MESSAGE_TOGGLE).write(1);
                display_script_stat_update_message(0, 0x15, g::<u32>(0xB78F6C).read());
            }
        } else {
            decrease(STAT_MUSCLE, 0xB78F70);
            g::<u8>(FAT_MESSAGE_TOGGLE).write(3);
            display_script_stat_update_message(0, 0x17, g::<u32>(0xB78F70).read());
        }
    } else {
        let n = (step_ms() as u32).wrapping_mul(fat_burned) / 10;
        let u = g::<u32>(FAT_UNITS);
        u.write(u.read().wrapping_add(n));
    }
    let thr = x(gf(0xB78FC4)).fmul_m(1000.0);
    if matches!(thr.fcom(units(MUSCLE_UNITS)), Cmp::Less) {
        increment_stat_call(0x18, g::<u32>(0xB78F3C).read());
        display_script_stat_update_message(1, 0x18, g::<u32>(0xB78F3C).read());
        g::<u32>(MUSCLE_UNITS).write(0);
    } else {
        let u = g::<u32>(MUSCLE_UNITS);
        u.write(u.read().wrapping_add(step_ms() as u32));
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE_FAT_AND_MUSCLE_STATS_PORT,
    HOOK_SET_STAT_VALUE,
    HOOK_MODIFY_STAT,
    HOOK_PROCESS_REACTION_STATS_ON_DECREMENT,
    HOOK_PROCESS_REACTION_STATS_ON_INCREMENT,
    HOOK_DECREMENT_STAT,
    HOOK_UPDATE_STATS_WHEN_SPRINTING,
    HOOK_UPDATE_STATS_WHEN_RUNNING,
    HOOK_UPDATE_STATS_WHEN_FIGHTING,
    HOOK_UPDATE_STATS_ADD_TO_HEALTH,
    HOOK_GET_STAT_ID,
    HOOK_IS_STAT_FLOAT,
    HOOK_GET_STAT_VALUE,
    HOOK_GET_TIMES_MISSION_ATTEMPTED,
    HOOK_REGISTER_MISSION_ATTEMPTED,
    HOOK_REGISTER_MISSION_PASSED,
    HOOK_GET_FULL_FAVORITE_RADIO_STATION_LIST,
    HOOK_FIND_MOST_FAVORITE_RADIO_STATION,
    HOOK_FIND_LEAST_FAVORITE_RADIO_STATION,
    HOOK_FIND_CRIMINAL_RATING_NUMBER,
    HOOK_GET_PERCENTAGE_PROGRESS,
    HOOK_CONVERT_TO_MINS,
    HOOK_CONVERT_TO_SECS,
    HOOK_SAFE_TO_SHOW_THIS_STAT,
    HOOK_CHECK_FOR_THRESHOLD,
    HOOK_IS_STAT_CAPPED,
];
