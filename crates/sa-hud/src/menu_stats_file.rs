//! `CMenuManager::SaveStatsToFile` (0x57DDE0, PORT-MENU): the stats page written as `stats.html` into the
//! user documents folder. Ported from the 1.0 US bytes (gta-reversed has no body for it). File output
//! goes through the CRT's `fopen` / `fprintf` / `fclose` (`orig!`; natively sa-game's CRT replacements
//! over sa-platform's VFS, text mode: `\n` is written as `\r\n`).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, wr, x, xi};
use crate::font_print::file_set_dir;
use crate::menu_draw::{gxt_char_strcpy, stats_construct_stat_line, stats_find_criminal_rating_number, stats_find_criminal_rating_string};
use crate::menu_mgr::text_get;
use crate::text_load::{file_set_dir_my_documents, gxt_char_to_ascii_call};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `TheText`
const THE_TEXT: usize = 0xC1B340;
/// `CStats::LastMissionPassedName` (char[8])
const LAST_MISSION_PASSED_NAME: usize = 0xB78A00;
/// `gGxtString` (0xC1B100), `gGxtString2` (0xC1AED8)
const GXT_STRING: usize = 0xC1B100;
const GXT_STRING2: usize = 0xC1AED8;
/// `CStats` current stat id word (0xB794CC): non-zero while the line just built is a percentage stat
const STATS_CURRENT_ID: usize = 0xB794CC;

// .rdata strings
/// `"ITBEG"`, `"stats.html"`, `"w"`, `""`
const S_ITBEG: usize = 0x8663B0;
const S_STATS_HTML: usize = 0x8663A0;
const S_W: usize = 0x8663AC;
const S_EMPTY: usize = 0x858B54;
/// GXT keys `"FEH_STA"`, `"FES_DAT"`, `"FES_CMI"`, `"CRIMRA"`
const K_FEH_STA: usize = 0x86613C;
const K_FES_DAT: usize = 0x866000;
const K_FES_CMI: usize = 0x865F64;
const K_CRIMRA: usize = 0x865E14;
/// The page titles of the eight stat pages (jump table 0x57E21C): `FES_PLA`, `FES_MON`, `FES_WEA`,
/// `FES_GAN`, `FES_CRI`, `FES_ACH`, `FES_MIS`, `FES_MSC`
const PAGE_KEYS: [usize; 8] = [0x865C0C, 0x865C04, 0x865BFC, 0x865BF4, 0x865BEC, 0x865BE4, 0x865BDC, 0x865BD4];
/// `"%s"`, `" "`, `"%0.0f%%"`
const F_S: usize = 0x85C714;
const F_SPACE: usize = 0x865A24;
const F_PERCENT: usize = 0x865990;
/// The fixed HTML lines (format strings without arguments), in the order the bytes print them
const HEAD: [usize; 8] = [0x86636C, 0x866310, 0x8662C0, 0x866298, 0x866228, 0x8661B8, 0x866180, 0x866144];
const AFTER_TITLE: [usize; 3] = [0x8660A8, 0x866040, 0x866008];
const AFTER_MISSION: [usize; 2] = [0x865E80, 0x865E20];
const PAGE_HEAD: [usize; 2] = [0x865CA8, 0x865C18];
const PAGE_TITLE_END: usize = 0x865B48;
const LINE_SEP: usize = 0x865AC8;
const LINE_HEAD: usize = 0x865A28;
const LINE_MID: usize = 0x865998;
const TAIL: [usize; 6] = [0x865AC8, 0x865900, 0x865858, 0x8657C0, 0x865720, 0x865708];
/// Formats with arguments: the title `"%s</font></strong><br><font\n"`, the date line (one `%s`), the
/// last mission `"%s</font><br>        %s: </strong>"` and `"%s<strong><br></strong> </font></p></td></tr>\n"`,
/// the criminal rating title (one `%s`) and `"%s (%d)</font></td> ..."`
const F_TITLE: usize = 0x86611C;
const F_DATE: usize = 0x865F70;
const F_MISSION: usize = 0x865F40;
const F_MISSION_NAME: usize = 0x865F10;
const F_RATING_TITLE: usize = 0x865DA0;
const F_RATING: usize = 0x865D28;

/// 0.001 (0x858CDC), 100.0 (0x858628)
const K_0_001: f32 = f32::from_bits(0x3A83_126F);
const K_100: f32 = f32::from_bits(0x42C8_0000);

orig! {
    /// CRT `_strdate(char buf[9])` (0x823A8E): `MM/DD/YY` from `GetLocalTime`
    fn strdate(buf: GPtr<u8>) -> GPtr<u8> = cdecl 0x823A8E => unported;
    /// CRT `fopen` (0x8232D8)
    fn fopen(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x8232D8 => unported;
    /// CRT `fclose` (0x82318B)
    fn fclose(f: GPtr<u8>) -> i32 = cdecl 0x82318B => unported;
    /// CRT `fprintf(f, fmt)` (0x823A30)
    fn fprintf0(f: GPtr<u8>, fmt: GPtr<u8>) -> i32 = cdecl 0x823A30 => unported;
    /// CRT `fprintf(f, fmt, a)` (0x823A30)
    fn fprintf1(f: GPtr<u8>, fmt: GPtr<u8>, a: u32) -> i32 = cdecl 0x823A30 => unported;
    /// CRT `fprintf(f, fmt, a, b)` (0x823A30)
    fn fprintf2(f: GPtr<u8>, fmt: GPtr<u8>, a: u32, b: u32) -> i32 = cdecl 0x823A30 => unported;
    /// CRT `_strupr(char*)` (0x836FBA)
    fn strupr(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x836FBA => unported;
    /// CRT `atol` (0x82258E)
    fn atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// CRT `sprintf(buf, fmt, double)` (0x821BB5; the double as two stack words)
    fn sprintf_f64(buf: GPtr<u8>, fmt: GPtr<u8>, lo: u32, hi: u32) -> i32 = cdecl 0x821BB5 => unported;
}

fn p(a: usize) -> GPtr<u8> {
    g(a)
}

/// `GxtCharToAscii(TheText.Get(key), 0)`
fn text_ascii(key: usize) -> GPtr<u8> {
    gxt_char_to_ascii_call(text_get(g(THE_TEXT), p(key)), 0)
}

/// `CMenuManager::SaveStatsToFile()` (0x57DDE0, thiscall):
///
/// 1. `SetDirMyDocuments()`, `_strdate(date)`; the last mission's text = `TheText.Get(LastMissionPassedName)`
///    (`"ITBEG"` when the name is empty).
/// 2. `f = fopen("stats.html", "w")`; when it opens: the HTML head (8 fixed lines), the title
///    `_strupr(GxtCharToAscii(Get("FEH_STA")))`, 3 fixed lines, the date line (`FES_DAT`), `FES_CMI` with
///    the date, the last mission's name upper-cased, 2 fixed lines, the criminal rating title (`CRIMRA`),
///    `FindCriminalRatingString` copied into `gGxtString` and printed with `FindCriminalRatingNumber`.
///    Then for each of the 8 stat pages: `n = ConstructStatLine(99999, page)`, 2 fixed lines, the page
///    title (`FES_PLA` .. `FES_MSC`), and for each line `i < n`: `ConstructStatLine(i, page)`, the line
///    name (`gGxtString` as ASCII; a separator line first and the name, else `" "`), and the value
///    (`gGxtString2` as ASCII; for a percentage stat (word 0xB794CC set) rewritten in place as
///    `"%0.0f%%"` of `min(atol(v) * 0.001 * 100, 100)` (x87; NaN and >= 100 give 100), every `'|'`
///    replaced by 0xBA). 6 fixed closing lines, `fclose`.
/// 3. `SetDir("")`, helper text +0x1B18 = 6 (the "stats saved" message), its fade +0x1AEC = 300.
///
/// Uninitialised stack: the original passes `ConstructStatLine` its page as a dword whose upper three
/// bytes are residue (the callee reads the byte); the port passes the zero-extended byte.
#[hook(addr = 0x57DDE0, conv = thiscall, name = "CMenuManager::SaveStatsToFile", expect = "55 8B EC 83 E4 F8",
       unverified = "menu-2", route = "split", hazards = "heap-writes",
       recs = "menus-map-controls",
       note = "writes stats.html in the user documents folder (CRT fopen / fprintf / fclose); _strdate reads the wall-clock date")]
pub fn save_stats_to_file(this: GPtr<u8>) {
    file_set_dir_my_documents();
    let mut date = [0u8; 12];
    let date_p = GPtr::from_raw(date.as_mut_ptr());
    strdate(date_p);
    let mission_key = if g::<u8>(LAST_MISSION_PASSED_NAME).read() != 0 { p(LAST_MISSION_PASSED_NAME) } else { p(S_ITBEG) };
    let mission = text_get(g(THE_TEXT), mission_key);
    let f = fopen(p(S_STATS_HTML), p(S_W));
    if !f.is_null() {
        write_stats(f, date_p, mission);
    }
    file_set_dir(p(S_EMPTY));
    wr::<u32>(this, 0x1B18, 6);
    wr::<u32>(this, 0x1AEC, 300);
}

/// Step 2 of [`save_stats_to_file`] on the open file.
fn write_stats(f: GPtr<u8>, date: GPtr<u8>, mission: GPtr<u8>) {
    let put = |fmt: usize| {
        fprintf0(f, p(fmt));
    };
    HEAD.iter().for_each(|&s| put(s));
    let title = strupr(text_ascii(K_FEH_STA));
    fprintf1(f, p(F_TITLE), title.addr() as u32);
    AFTER_TITLE.iter().for_each(|&s| put(s));
    fprintf1(f, p(F_DATE), text_ascii(K_FES_DAT).addr() as u32);
    fprintf2(f, p(F_MISSION), date.addr() as u32, text_ascii(K_FES_CMI).addr() as u32);
    let name = strupr(gxt_char_to_ascii_call(mission, 0));
    fprintf1(f, p(F_MISSION_NAME), name.addr() as u32);
    AFTER_MISSION.iter().for_each(|&s| put(s));
    fprintf1(f, p(F_RATING_TITLE), text_ascii(K_CRIMRA).addr() as u32);
    gxt_char_strcpy(p(GXT_STRING), stats_find_criminal_rating_string());
    let rating = gxt_char_to_ascii_call(p(GXT_STRING), 0);
    let number = stats_find_criminal_rating_number();
    fprintf2(f, p(F_RATING), rating.addr() as u32, number as u32);

    for (page, &key) in PAGE_KEYS.iter().enumerate() {
        let page_arg = page as u32;
        let lines = stats_construct_stat_line(99999, page_arg);
        PAGE_HEAD.iter().for_each(|&s| put(s));
        fprintf1(f, p(F_S), text_ascii(key).addr() as u32);
        put(PAGE_TITLE_END);
        for i in 0..lines.max(0) {
            stats_construct_stat_line(i, page_arg);
            let name = gxt_char_to_ascii_call(p(GXT_STRING), 0);
            if name.read() != 0 {
                put(LINE_SEP);
            }
            put(LINE_HEAD);
            if name.read() != 0 {
                fprintf1(f, p(F_S), name.addr() as u32);
            } else {
                put(F_SPACE);
            }
            put(LINE_MID);
            let value = gxt_char_to_ascii_call(p(GXT_STRING2), 0);
            if g::<u16>(STATS_CURRENT_ID).read() != 0 {
                let n = atol(value);
                let mut v = xi(n).fmul_m(K_0_001).fmul_m(K_100);
                if v.fcom(x(K_100)).test_parity(0x05) {
                    v = x(K_100);
                }
                let d = v.to_f64().to_bits();
                sprintf_f64(value, p(F_PERCENT), d as u32, (d >> 32) as u32);
            }
            let mut q = value;
            while q.read() != 0 {
                if q.read() == b'|' {
                    q.write(0xBA);
                }
                q = q.byte_add(1);
            }
            fprintf1(f, p(F_S), value.addr() as u32);
        }
    }
    TAIL.iter().for_each(|&s| put(s));
    fclose(f);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SAVE_STATS_TO_FILE];

