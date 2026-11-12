//! SAVE-1: the stats strings of gta_sa.exe 1.0 US the stats page and `SaveStatsToFile` print:
//! `CStats::FindCriminalRatingString` (0x55A210) and `CStats::ConstructStatLine` (0x55A780). Ported from the
//! bytes (docs/re/specs/WORLD/CStats.md as the map; every handler re-read against the disassembly;
//! docs/arch/save-port.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use super::common::{ftol32, g, x, xi};

orig! {
    /// `CStats::FindCriminalRatingNumber()` (0x559080)
    fn find_criminal_rating_number() -> i32 = cdecl 0x559080 => super::stats::find_criminal_rating_number;
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
}

/// `TheText`
const THE_TEXT: usize = 0xC1B340;

/// The non-negative rating bands: (rating below, GXT key address). The band below 0x514 and the one below
/// 0x7A120 depend on a stat (see [`find_criminal_rating_string`]); their entries hold 0.
const BANDS: [(i32, usize); 50] = [
    (0x14, 0x864154),
    (0x32, 0x86414C),
    (0x4B, 0x864144),
    (0x64, 0x86413C),
    (0x78, 0x864134),
    (0x96, 0x86412C),
    (0xC8, 0x864124),
    (0xF0, 0x86411C),
    (0x10E, 0x864114),
    (0x12C, 0x86410C),
    (0x14F, 0x864104),
    (0x172, 0x8640FC),
    (0x190, 0x8640F4),
    (0x1C2, 0x8640EC),
    (0x1F4, 0x8640E4),
    (0x226, 0x8640DC),
    (0x258, 0x8640D4),
    (0x262, 0x8640CC),
    (0x28A, 0x8640C4),
    (0x2BC, 0x8640BC),
    (0x352, 0x8640B4),
    (0x3E8, 0x8640AC),
    (0x3ED, 0x8640A4),
    (0x47E, 0x86409C),
    (0x514, 0),
    (0x5DC, 0x86408C),
    (0x6A4, 0x864084),
    (0x7D0, 0x86407C),
    (0x834, 0x864074),
    (0x8FC, 0x86406C),
    (0x9C4, 0x864064),
    (0xABE, 0x86405C),
    (0xBB8, 0x864054),
    (0xDAC, 0x86404C),
    (0xFA0, 0x864044),
    (0x1388, 0x86403C),
    (0x1D4C, 0x864034),
    (0x2710, 0x86402C),
    (0x4E20, 0x864024),
    (0x7530, 0x86401C),
    (0x9C40, 0x864014),
    (0xC350, 0x86400C),
    (0xFDE8, 0x864004),
    (0x13880, 0x863FFC),
    (0x186A0, 0x863FF4),
    (0x249F0, 0x863FEC),
    (0x30D40, 0x863FE4),
    (0x493E0, 0x863FDC),
    (0x5B8D8, 0x863FD4),
    (0x7A120, 0),
];

/// The GXT key `FindCriminalRatingString` looks up for a rating (pure apart from the two stat reads).
pub fn criminal_rating_key(r: i32) -> usize {
    if r < 0 {
        return match r {
            -499..=-1 => 0x86417C,
            -1999..=-500 => 0x864174,
            -3999..=-2000 => 0x86416C,
            -5999..=-4000 => 0x864164,
            _ => 0x86415C,
        };
    }
    for (below, key) in BANDS {
        if r >= below {
            continue;
        }
        if key != 0 {
            return key;
        }
        if below == 0x514 {
            // fild [0xB79034]; fcomp dword [0x858B50]; test ah, 0x41: greater (ordered) -> 0x864094
            let stat = f64::from(g::<i32>(0xB79034).read());
            let c = f64::from(g::<f32>(0x858B50).read());
            return if stat > c { 0x864094 } else { 0x86409C };
        }
        // fild [0xB790C4]; fmul [0x863FD0]; fmul [0x859044]; _ftol; > 10 -> 0x863FC8
        let v = xi(g::<i32>(0xB790C4).read()).fmul_m(g::<f32>(0x863FD0).read()).fmul_m(g::<f32>(0x859044).read());
        return if ftol32(v) > 10 { 0x863FC8 } else { 0x863FD4 };
    }
    let focus = u32::from(g::<u8>(0xB7CD74).read());
    if r >= 0xF4240 && g::<i32>((0xB7CE54 + focus * 0x190) as usize).read() > 0x98_9680 {
        0x863FC0
    } else {
        0x863FB8
    }
}

/// `CStats::FindCriminalRatingString()` (0x55A210): `TheText.Get(key)` for `FindCriminalRatingNumber()`: five
/// negative bands (> -500, > -2000, > -4000, > -6000, else), fifty non-negative ones (`BANDS`; below 0x514 the
/// key depends on whether stat 0xB79034 exceeds the float at 0x858B50, below 0x7A120 on whether
/// `(int)(stat 0xB790C4 * f[0x863FD0] * f[0x859044])` exceeds 10), and above them the money of the focused
/// player (+0xBC) above 10,000,000 with a rating of at least 1,000,000.
#[hook(addr = 0x55A210, conv = cdecl, name = "CStats::FindCriminalRatingString", expect = "E8 6B EE FF FF",
       unverified = "save-stats-1", route = "L2", hazards = "none", recs = "menus-hud,menus-map-controls")]
pub fn find_criminal_rating_string() -> GPtr<u8> {
    let key = criminal_rating_key(find_criminal_rating_number());
    text_get(g(THE_TEXT), g(key))
}

// ------------------------------------------------------------------------- ConstructStatLine ----

orig! {
    /// CRT `sprintf(buf, fmt, int)` (0x821BB5)
    fn sprintf_i(buf: GPtr<u8>, fmt: GPtr<u8>, v: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt)` (0x821BB5)
    fn sprintf_0(buf: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, double)` (0x821BB5)
    fn sprintf_d(buf: GPtr<u8>, fmt: GPtr<u8>, v: f64) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, double, char*)` (0x821BB5)
    fn sprintf_ds(buf: GPtr<u8>, fmt: GPtr<u8>, v: f64, s: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `sprintf(buf, fmt, int, char*, int)` (0x821BB5)
    fn sprintf_isi(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32, s: GPtr<u8>, b: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    fn get_stat_value(stat: u32) -> X87 = reg("args = stack") 0x558E40 => unported;
    /// `CStats::GetPercentageProgress()` (0x5591E0, ST0)
    fn get_percentage_progress() -> X87 = reg("args = stack") 0x5591E0 => unported;
    /// `CStats::ConvertToMins(int32)` (0x559540)
    fn convert_to_mins(v: i32) -> i32 = cdecl 0x559540 => super::stats::convert_to_mins;
    /// `CStats::ConvertToSecs(int32)` (0x559560)
    fn convert_to_secs(v: i32) -> i32 = cdecl 0x559560 => super::stats::convert_to_secs;
    /// `CStats::BuildStatLine(const char* key, void* p1, int32 metrics, void* p2, int32 type)` (0x559230)
    fn build_stat_line(key: GPtr<u8>, p1: GPtr<u8>, metrics: i32, p2: GPtr<u8>, ty: i32) = cdecl 0x559230 => unported;
    /// `CStats::PopulateFavoriteRadioStationList()` (0x558EC0)
    fn populate_favorite_radio_station_list() -> u8 = cdecl 0x558EC0 => unported;
    /// `CStats::FindMostFavoriteRadioStation()` (0x558FA0)
    fn find_most_favorite_radio_station() -> i32 = cdecl 0x558FA0 => unported;
    /// `CStats::FindLeastFavoriteRadioStation()` (0x559010)
    fn find_least_favorite_radio_station() -> i32 = cdecl 0x559010 => unported;
    /// `CAudioEngine::GetRadioStationNameKey(int32, char*)` (0x507010)
    fn get_radio_station_name_key(this: GPtr<u8>, id: i32, out: GPtr<u8>) = thiscall 0x507010 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60, AL)
    fn get_weapon_skill(ped: GPtr<u8>, ty: u32) -> u8 = thiscall 0x5E3B60 => unported;
    /// `CLocalisation::GermanGame()` (0x56D200)
    fn german_game() -> u8 = cdecl 0x56D200 => unported;
    /// `CLocalisation::Metric()` (0x56D220)
    fn metric() -> u8 = cdecl 0x56D220 => unported;
    /// `GxtCharToAscii(const GxtChar*, uint8 start)` (0x69F7E0)
    fn gxt_char_to_ascii(s: GPtr<u8>, start: u32) -> GPtr<u8> = cdecl 0x69F7E0 => unported;
    /// `GxtCharStrcpy(GxtChar* dst, const GxtChar* src)` (0x718660)
    fn gxt_char_strcpy(dst: GPtr<u8>, src: GPtr<u8>) = cdecl 0x718660 => unported;
    /// 0x719240: removes every `~...~` token from a GXT string in place
    fn strip_tokens(s: GPtr<u8>) = cdecl 0x719240 => unported;
    /// `Common::AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
}

/// `gString` / `gString2` / `gGxtString` / `gGxtString2`
const G_STRING: usize = 0xB71670;
const G_STRING2: usize = 0xB71510;
const G_GXT_STRING: usize = 0xC1B100;
const G_GXT_STRING2: usize = 0xC1AED8;
/// `CStats::m_ThisStatIsABarChart` (int16)
const BAR_CHART: usize = 0xB794CC;
/// `CStats::StatTypesFloat` (82 floats) / the int stats (`0xB78E20 + 4 * id`)
const FLOATS: usize = 0xB79380;
const INTS: usize = 0xB78E20;
/// float 0.0 (0x858B50), 1/60000 (0x863FD0), 3.3333333 (0x859F60), 100.0 (0x858628), 0.4536 (0x863F0C)
const F_ZERO: usize = 0x858B50;
const F_PER_MINUTE: usize = 0x863FD0;
const F_FEET: usize = 0x859F60;
const F_HUNDRED: usize = 0x858628;
const F_KGS: usize = 0x863F0C;

fn s(a: usize) -> GPtr<u8> {
    g(a)
}

/// `fild` / `fld` of a stat as the visibility tests load it: float stats below 82, else the int.
fn stat_x87(id: u16) -> X87 {
    if id < 0x52 {
        x(g::<f32>(FLOATS + 4 * usize::from(id)).read())
    } else {
        xi(g::<i32>(INTS + 4 * usize::from(id)).read())
    }
}

/// `fcomp dword [0.0]; test ah, 0x41`: the value is greater than 0 (ordered).
fn positive(v: X87) -> bool {
    v.to_f64() > f64::from(g::<f32>(F_ZERO).read())
}

/// The int stat `I[id]` through `fild` and `_ftol`.
fn int_stat(id: usize) -> i32 {
    ftol32(xi(g::<i32>(INTS + 4 * id).read()))
}

/// The STD tail (0x55A8D4): `GxtCharStrcpy(gGxtString, TheText.Get(gString))`, the token strip,
/// `AsciiToGxtChar(gString2, gGxtString2)`; returns 0.
fn std_tail() -> i32 {
    let t = text_get(g(THE_TEXT), s(G_STRING));
    gxt_char_strcpy(s(G_GXT_STRING), t);
    strip_tokens(s(G_GXT_STRING));
    ascii_to_gxt_char(s(G_STRING2), s(G_GXT_STRING2));
    0
}

/// `gString2 = ""` then the STD tail (0x55B60D, the header line of a multi-line stat).
fn hdr() -> i32 {
    g::<u8>(G_STRING2).write(0);
    std_tail()
}

/// `INT(v)` (0x55B087 through the protection chunk 0x401240): `sprintf(gString2, "%d", v)`, STD tail.
fn int_line(v: i32) -> i32 {
    g::<u8>(G_STRING2).write(0);
    sprintf_i(s(G_STRING2), s(0x859508), v);
    std_tail()
}

/// `sprintf(gString2, fmt, (double)v)` after `gString2[0] = 0` (0x55A8C0), STD tail.
fn double_line(fmt: usize, v: f64) -> i32 {
    g::<u8>(G_STRING2).write(0);
    sprintf_d(s(G_STRING2), s(fmt), v);
    std_tail()
}

/// TXT(key) (0x55B68F after the callers' `gGxtString[0] = 0`): `GxtCharStrcpy(gGxtString2, TheText.Get(key))`.
fn txt(key: GPtr<u8>) -> i32 {
    g::<u8>(G_GXT_STRING).write(0);
    let t = text_get(g(THE_TEXT), key);
    gxt_char_strcpy(s(G_GXT_STRING2), t);
    0
}

/// `sprintf(buf, v < 10 ? fmt00 : v < 100 ? fmt0 : fmt, v)` (signed for ids, unsigned bytes for weapons).
fn sprintf_numbered(buf: usize, f00: usize, f0: usize, f: usize, v: i32, small: bool) {
    let fmt = if small { f00 } else if v < 100 { f0 } else { f };
    sprintf_i(s(buf), s(fmt), v);
}

/// The " %d %s %d" pair line (0x55AC1F): `A("FEST_OO")` built first, then `sprintf(gString2, " %d %s %d",
/// first, A, second)`, STD tail.
fn pair_line(first: i32, second: i32) -> i32 {
    g::<u8>(G_STRING2).write(0);
    let a = gxt_char_to_ascii(text_get(g(THE_TEXT), s(0x863F50)), 0);
    sprintf_isi(s(G_STRING2), s(0x863F24), first, a, second);
    std_tail()
}

/// A time line through `BuildStatLine(gString, &a, 0, &b, 1)`; returns 0.
fn time_line(a: i32, b: i32) -> i32 {
    let mut v = [a, b];
    let p = GPtr::<u8>::from_raw(v.as_mut_ptr().cast());
    build_stat_line(s(G_STRING), p, 0, p.byte_add(4), 1);
    0
}

/// `(t / 60, t % 60)` as `cdq; idiv 60`.
fn div60(t: i32) -> (i32, i32) {
    (t.wrapping_div(60), t.wrapping_rem(60))
}

/// The stat tables of the 8 categories (6-byte entries `{i16 id, u8 type, u8 alwaysShow, u8 special, pad}`,
/// ended by id -99).
const TABLES: [usize; 8] = [0x8CD8A0, 0x8CD938, 0x8CD9B0, 0x8CD9D8, 0x8CDA70, 0x8CDAC8, 0x8CDBF8, 0x8CDE08];

/// `CStats::ConstructStatLine(int32 line, uint8 category)` (0x55A780; entry `jmp 0x40211F`, which loads the
/// category byte and returns to 0x55A785): walks the category's stat table counting the visible lines `n`;
/// the line `n == line` is built into `gGxtString` (label) / `gGxtString2` (value) and 0 returned; after the
/// table the line count is returned. Every handler follows the bytes (docs/re/specs/WORLD/CStats.md and
/// docs/arch/save-port.md); notably type 5 in metric units prints the low dword of a double for "%dkgs"
/// (original bug: "0kgs"), the bar-chart id is stored before the line test, the weapon-skill and radio
/// lines call their game functions even when the next line is not requested.
#[hook(addr = 0x55A780, conv = cdecl, name = "CStats::ConstructStatLine", expect = "E9 9A 79 EA FF 83 EC 18",
       unverified = "save-stats-1", route = "L2", hazards = "x87-transcendental", recs = "menus-hud,menus-map-controls",
       note = "x87: fmul / fsub with the 24-bit precision model; the category argument slot is reused as a temp (not modelled)")]
pub fn construct_stat_line(line: i32, category: u32) -> i32 {
    let table = TABLES[usize::from(category as u8).min(7)];
    let mut n: i32 = 0;
    let mut idx: u16 = 0;
    loop {
        let e = g::<u8>(table + 6 * usize::from(idx));
        let id = e.cast::<i16>().read();
        if id == -99 {
            return n;
        }
        idx = idx.wrapping_add(1);
        match entry(e, id, line, &mut n) {
            Some(r) => return r,
            None => continue,
        }
    }
}

/// One table entry: `Some(0)` when the requested line was built, `None` to go on (n already counted).
fn entry(e: GPtr<u8>, id: i16, line: i32, n: &mut i32) -> Option<i32> {
    let uid = id as u16;
    if e.byte_add(3).read() == 0 && !positive(stat_x87(uid)) {
        return None;
    }
    let edi = i32::from(id);
    sprintf_numbered(G_STRING, 0x864300, 0x8642F8, 0x8642F0, edi, edi < 10);
    let low = e.read();
    if german_game() != 0 && matches!(low, 167 | 168 | 177 | 205..=209) {
        return None;
    }
    g::<i16>(BAR_CHART).write(0);
    // `cmp esi, ebp; jne n++`: the line test of every handler
    macro_rules! line {
        () => {
            if *n != line {
                *n += 1;
                return None;
            }
        };
    }
    if e.byte_add(4).read() == 0 {
        let ty = e.byte_add(2).read();
        match ty {
            10 => {
                g::<i16>(BAR_CHART).write(id);
                line!();
                return Some(int_line(ftol32(get_stat_value(edi as u32))));
            }
            5 => {
                line!();
                let w = ftol32(get_stat_value(edi as u32));
                g::<u8>(G_STRING2).write(0);
                if metric() == 0 {
                    sprintf_i(s(G_STRING2), s(0x863EFC), w);
                } else {
                    let v = xi(w).fmul_m(g::<f32>(F_KGS).read()).to_f64();
                    sprintf_d(s(G_STRING2), s(0x863F04), v);
                }
                return Some(std_tail());
            }
            3 => {
                line!();
                return Some(double_line(0x863EF4, get_stat_value(edi as u32).to_f64()));
            }
            4 => {
                line!();
                let v = ftol32(get_stat_value(edi as u32));
                g::<u8>(G_STRING2).write(0);
                sprintf_i(s(G_STRING2), s(0x863F10), v);
                return Some(std_tail());
            }
            9 => {
                line!();
                let mins = convert_to_mins(ftol32(get_stat_value(edi as u32)));
                let secs = convert_to_secs(ftol32(get_stat_value(edi as u32)));
                return Some(time_line(mins, secs));
            }
            1 => {
                line!();
                return Some(double_line(0x863F1C, get_stat_value(edi as u32).to_f64()));
            }
            6 => {
                line!();
                let v = get_stat_value(edi as u32).to_f32();
                g::<u8>(G_STRING2).write(0);
                let a = gxt_char_to_ascii(text_get(g(THE_TEXT), s(0x863EDC)), 0);
                sprintf_ds(s(G_STRING2), s(0x863ED4), f64::from(v), a);
                return Some(std_tail());
            }
            7 => {
                if metric() != 0 {
                    line!();
                    return Some(double_line(0x863EEC, get_stat_value(edi as u32).to_f64()));
                }
                line!();
                let v = get_stat_value(edi as u32).fmul_m(g::<f32>(F_FEET).read()).to_f64();
                return Some(double_line(0x863EE4, v));
            }
            _ => {
                line!();
                let v = if uid < 0x52 {
                    ftol32(x(g::<f32>(FLOATS + 4 * usize::from(uid)).read()))
                } else {
                    ftol32(xi(g::<i32>(INTS + 4 * usize::from(uid)).read()))
                };
                return Some(int_line(v));
            }
        }
    }
    // special handlers
    if edi > 0x101 {
        return special_high(edi, line, n);
    }
    if edi >= 0xFC {
        if !positive(stat_x87(uid)) {
            return None;
        }
        line!();
        return Some(double_line(0x863F14, get_stat_value(edi as u32).to_f64()));
    }
    if edi as u32 > 0xF3 {
        return None;
    }
    match edi {
        0 => {
            line!();
            Some(double_line(0x863F14, get_percentage_progress().to_f64()))
        }
        164 => {
            line!();
            let ped = g::<GPtr<u8>>(0xB7CD98).read();
            Some(int_line(ftol32(x(ped.byte_add(0x548).cast::<f32>().read()))))
        }
        213 | 174 | 175 => {
            line!();
            let second = match edi {
                213 => 0x1E,
                174 => 0xC,
                _ => 0x19,
            };
            Some(pair_line(int_stat(edi as usize), second))
        }
        231 | 241 | 243 => {
            line!();
            let a = int_stat(edi as usize);
            Some(pair_line(a, int_stat(edi as usize + 1)))
        }
        144 | 145 => {
            line!();
            let a = int_stat(edi as usize);
            Some(pair_line(a, g::<i32>(0xA9A89C).read()))
        }
        143 => {
            if *n == line {
                return Some(hdr());
            }
            let k = int_stat(143);
            *n += 1;
            let keys = [0x8642E8usize, 0x8642E0, 0x8642D8, 0x8642D0, 0x8642C8, 0x8642C0, 0x8642B8, 0x8642B0];
            let key = keys.get((k.wrapping_sub(1)) as u32 as usize).copied().unwrap_or(0x8642A8);
            line!();
            Some(txt(s(key)))
        }
        169 | 173 => {
            let t = ftol32(xi(g::<i32>(INTS + 4 * edi as usize).read()).fmul_m(g::<f32>(F_PER_MINUTE).read()));
            let (q, r) = div60(t);
            if edi == 173 && q <= 0 && r <= 0 {
                return None;
            }
            line!();
            Some(time_line(q, r))
        }
        _ => None,
    }
}

/// The special handlers of ids 0x140..0x151 (jump table 0x55B898); other ids above 0x101 are skipped.
fn special_high(edi: i32, line: i32, n: &mut i32) -> Option<i32> {
    macro_rules! line {
        () => {
            if *n != line {
                *n += 1;
                return None;
            }
        };
    }
    match edi {
        320 => {
            let m = ((u64::from(0x45E7_B273u32) * u64::from(g::<u32>(0xB7CB84).read())) >> 32) as u32 >> 14;
            let (h, mm) = div60(m as i32);
            line!();
            Some(time_line(h, mm))
        }
        322 => {
            line!();
            Some(pair_line(g::<i32>(0xA9AD74).read(), g::<i32>(0xA9AD70).read()))
        }
        323 => {
            let (mut best, mut gang) = (0u32, 0i32);
            for i in 7..15usize {
                let v = g::<u32>(0xB78E90 + 4 * i).read();
                if v > best {
                    gang = i as i32;
                    best = v;
                }
            }
            if gang == 0 {
                return None;
            }
            if *n == line {
                return Some(hdr());
            }
            *n += 1;
            let keys = [0x8642A0usize, 0x864298, 0x864290, 0x864288, 0x864280, 0x864278, 0x864270, 0x864268];
            let Some(&key) = keys.get((gang - 7) as u32 as usize) else {
                return None;
            };
            line!();
            Some(txt(s(key)))
        }
        324 => {
            line!();
            let mut sum = 0i32;
            for i in 7..16usize {
                sum = sum.wrapping_add(g::<i32>(0xB78E90 + 4 * i).read());
            }
            Some(int_line(sum))
        }
        325 => {
            line!();
            Some(int_line(g::<i32>(0xB78EE0).read()))
        }
        326 | 327 => {
            if populate_favorite_radio_station_list() != 0 {
                return None;
            }
            if *n == line {
                return Some(hdr());
            }
            let mut key = [0u8; 8];
            let kp = GPtr::<u8>::from_raw(key.as_mut_ptr());
            *n += 1;
            let station = if edi == 326 { find_most_favorite_radio_station() } else { find_least_favorite_radio_station() };
            get_radio_station_name_key(g(0xB6BC90), station, kp);
            line!();
            Some(txt(kp))
        }
        328 => {
            let ped = find_player_ped(-1);
            let slot = i32::from(ped.byte_add(0x718).read() as i8);
            let w16 = ped.byte_add((slot * 0x1C + 0x5A0) as isize).cast::<u16>().read();
            let al: u8 = if w16 == 0x20 {
                0x1C
            } else if !(0x16..=0x20).contains(&w16) {
                return None;
            } else {
                w16 as u8
            };
            let mut b = al.wrapping_sub(0x15);
            if *n == line {
                return Some(int_line(ftol32(get_stat_value(u32::from(b) + 0x44))));
            }
            let ped = find_player_ped(-1);
            *n += 1;
            let slot = i32::from(ped.byte_add(0x718).read() as i8);
            let w32 = ped.byte_add((slot * 0x1C + 0x5A0) as isize).cast::<u32>().read();
            if w32 == 0x20 {
                b = 0xB;
                sprintf_i(s(G_STRING), s(0x8641C8), i32::from(b));
            } else {
                sprintf_numbered(G_STRING, 0x8641BC, 0x8641C8, 0x8641B4, i32::from(b), b < 10);
            }
            line!();
            Some(txt(s(G_STRING)))
        }
        329 => {
            if *n == line {
                return Some(hdr());
            }
            *n += 1;
            for i in 1u16..11 {
                sprintf_numbered(G_STRING, 0x8641BC, 0x8641C8, 0x8641B4, i32::from(i), i < 10);
                let skill = get_weapon_skill(find_player_ped(-1), u32::from(i) + 0x15) as i8;
                let key = match skill {
                    1 => 0x86419C,
                    2 => 0x8641A4,
                    _ => 0x8641AC,
                };
                sprintf_0(s(G_STRING2), s(key));
                if *n == line {
                    let t = text_get(g(THE_TEXT), s(G_STRING));
                    gxt_char_strcpy(s(G_GXT_STRING), t);
                    let t2 = text_get(g(THE_TEXT), s(G_STRING2));
                    gxt_char_strcpy(s(G_GXT_STRING2), t2);
                    return Some(0);
                }
                *n += 1;
            }
            None
        }
        330 => {
            let t = ftol32(xi(g::<i32>(INTS + 4 * 169).read()).fmul_m(g::<f32>(F_PER_MINUTE).read()));
            let (q, r) = div60(t);
            if q > 0 || r >= 5 {
                if *n == line {
                    return Some(hdr());
                }
                *n += 1;
            }
            let key = pilot_rank_key(q, r)?;
            line!();
            Some(txt(s(key)))
        }
        331..=333 => {
            let gang = g::<i32>(0x96A60C + 4 * edi as usize).read();
            let val = g::<i32>(0x96A600 + 4 * edi as usize).read();
            let j = edi - 331;
            if gang < 0 {
                return None;
            }
            sprintf_i(s(G_STRING2), s(0x864190), gang);
            sprintf_i(s(G_STRING), s(0x864184), j);
            let lab = text_get(g(THE_TEXT), s(G_STRING));
            if *n == line {
                gxt_char_strcpy(s(G_GXT_STRING), lab);
                let t = text_get(g(THE_TEXT), s(G_STRING2));
                gxt_char_strcpy(s(G_GXT_STRING2), t);
                return Some(0);
            }
            sprintf_i(s(G_STRING), s(0x859508), val);
            *n += 1;
            ascii_to_gxt_char(s(G_STRING), s(G_GXT_STRING2));
            if *n == line {
                g::<u8>(G_GXT_STRING).write(0);
                gxt_char_strcpy(s(G_GXT_STRING2), s(G_GXT_STRING2));
                return Some(0);
            }
            *n += 1;
            None
        }
        334 => {
            let mut d = x(g::<f32>(0xB7940C).read()).fsub_m(g::<f32>(0xB79414).read());
            if d.to_f64() < f64::from(g::<f32>(F_ZERO).read()) {
                d = x(g::<f32>(F_ZERO).read());
            }
            if *n != line {
                *n += 1;
                return None;
            }
            Some(double_line(0x863EF4, d.to_f64()))
        }
        337 => {
            line!();
            let v = x(g::<f32>(0x96AB9C).read()).fmul_m(g::<f32>(F_HUNDRED).read()).to_f64();
            Some(double_line(0x863F14, v))
        }
        _ => None,
    }
}

/// The pilot ranking key (0x55B0D1 on): `None` = skip the entry.
fn pilot_rank_key(q: i32, r: i32) -> Option<usize> {
    if q <= 0 {
        if r < 5 {
            return None;
        }
        if r < 10 {
            return Some(0x864260);
        }
        if r < 20 {
            return Some(0x864258);
        }
        if r < 30 {
            return Some(0x864250);
        }
    }
    for (qmax, key_neg, key_30) in [(1, 0x864248, 0x864240), (2, 0x864238, 0x864230), (3, 0x864228, 0x864220)] {
        if q <= qmax {
            if r < 0 {
                return Some(key_neg);
            }
            if r < 30 {
                return Some(key_30);
            }
        }
    }
    for (qmax, rmax, key) in [
        (4, 0, 0x864218),
        (5, 0, 0x864210),
        (10, 0, 0x864208),
        (0x14, 0, 0x864200),
        (0x19, 0, 0x8641F8),
        (0x1E, 0, 0x8641F0),
        (0x31, 2, 0x8641E8),
        (0x32, 0, 0x8641E0),
        (0x64, 0, 0x8641D8),
    ] {
        if q <= qmax && r < rmax {
            return Some(key);
        }
    }
    Some(0x8641D0)
}

pub const HOOKS: &[HookReg] = &[HOOK_FIND_CRIMINAL_RATING_STRING, HOOK_CONSTRUCT_STAT_LINE];

#[cfg(test)]
mod tests {
    use super::criminal_rating_key;

    #[test]
    fn negative_bands() {
        assert_eq!(criminal_rating_key(-1), 0x86417C);
        assert_eq!(criminal_rating_key(-500), 0x864174);
        assert_eq!(criminal_rating_key(-2000), 0x86416C);
        assert_eq!(criminal_rating_key(-6000), 0x86415C);
        assert_eq!(criminal_rating_key(0), 0x864154);
        assert_eq!(criminal_rating_key(0x13), 0x864154);
        assert_eq!(criminal_rating_key(0x14), 0x86414C);
        assert_eq!(criminal_rating_key(0x5B8D7), 0x863FD4);
    }
}
