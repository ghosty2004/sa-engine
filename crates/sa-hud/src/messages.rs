//! `CMessages` (PORT-HUD): brief (small) messages, big messages, the previous-brief history and the
//! GXT string helpers. Ported from the 1.0 US bytes; `docs/re/specs/HUD/CMessages.deviations.md` and
//! gta-reversed `Messages.cpp` as reading aids.
//!
//! `tMessage` (0x30): +0 text, +4 flags u16, +8 time, +0xC start time, +0x10 numbers[6], +0x28 string
//! to insert, +0x2C previous-brief flag. `tPreviousBrief` (0x20): +0 text, +4 numbers[6], +0x1C string.
//! Strings are `GxtChar` = 1 byte in 1.0 US. The original's stack buffers (0x190 / 0x320 bytes) are larger
//! here wherever a u16 index could run past them (the original then corrupts its own stack).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;
use sa_core::{hook, orig};

use crate::common::{g, rd, sprintf1, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CMessages::PreviousBriefs[20]` (`tPreviousBrief`, 0x20 each)
    pub static PREVIOUS_BRIEFS @ 0xC1A570: [u8; 20 * 0x20] = [0; 20 * 0x20];
    /// `CMessages::BriefMessages[8]` (`tMessage`, 0x30 each)
    pub static BRIEF_MESSAGES @ 0xC1A7F0: [u8; 8 * 0x30] = [0; 8 * 0x30];
    /// `CMessages::BigMessages[7]` (4 `tMessage` each, 0xC0)
    pub static BIG_MESSAGES @ 0xC1A970: [u8; 7 * 0xC0] = [0; 7 * 0xC0];
}

/// "%d"
const FMT_D: usize = 0x859508;
/// `ControlsManager` (`CControllerConfigManager`)
const CONTROLS_MANAGER: usize = 0xB70198;
/// `ControlsManager` +0x224: the 59 action names (40 bytes each) matched by "~k~~NAME~"
const ACTION_NAMES: usize = 0xB703BC;
/// byte compared with `Display`'s argument before the brief is shown
const DISPLAY_BRIEF_FLAG: usize = 0xA44488;

orig! {
    /// `CMessages::StringCopy(GxtChar*, const GxtChar*, uint16)` (0x69DB70)
    pub fn string_copy_call(dst: GPtr<u8>, src: GPtr<u8>, len: u32) = cdecl 0x69DB70 => string_copy;
    /// `CMessages::StringCompare(const GxtChar*, const GxtChar*, uint16)` (0x69DBD0)
    pub fn string_compare_call(a: GPtr<u8>, b: GPtr<u8>, len: u32) -> u8 = cdecl 0x69DBD0 => string_compare;
    /// `CMessages::ClearMessages(bool)` (0x69DCD0)
    pub fn clear_messages_call(flag: u32) = cdecl 0x69DCD0 => clear_messages;
    /// `CMessages::AddToPreviousBriefArray(text, n1..n6, string)` (0x69DD50)
    pub fn add_to_previous_brief_array_call(text: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, s: GPtr<u8>) = cdecl 0x69DD50 => add_to_previous_brief_array;
    /// `CMessages::InsertNumberInString(str, n1..n6, dst)` (0x69DE90)
    pub fn insert_number_in_string_call(s: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, dst: GPtr<u8>) = cdecl 0x69DE90 => insert_number_in_string;
    /// `CMessages::InsertStringInString(target, repl)` (0x69E040)
    pub fn insert_string_in_string_call(target: GPtr<u8>, repl: GPtr<u8>) = cdecl 0x69E040 => insert_string_in_string;
    /// `CMessages::InsertPlayerControlKeysInString(str)` (0x69E160)
    pub fn insert_player_control_keys_in_string_call(s: GPtr<u8>) = cdecl 0x69E160 => insert_player_control_keys_in_string;
    /// `CHud::GetRidOfAllHudMessages(bool)` (0x588A50)
    pub fn hud_get_rid_of_all_hud_messages(flag: u32) = cdecl 0x588A50 => unported;
    /// `CHud::SetMessage(const char*)` (0x588F60)
    pub fn hud_set_message(text: GPtr<u8>) = cdecl 0x588F60 => unported;
    /// `CHud::SetBigMessage(const char*, eMessageStyle)` (0x588FC0)
    pub fn hud_set_big_message(text: GPtr<u8>, style: u32) = cdecl 0x588FC0 => unported;
    /// `CControllerConfigManager::GetGxtStringOfCommandKeys(action, buf, size)` (0x5303D0)
    pub fn get_gxt_string_of_command_keys(this: GPtr<u8>, action: u32, buf: GPtr<u8>, size: u32) = thiscall 0x5303D0 => unported;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
}

/// `BriefMessages[i]`
fn brief(i: i32) -> GPtr<u8> {
    BRIEF_MESSAGES.ptr().cast::<u8>().byte_add(i as isize * 0x30)
}

/// `BigMessages[style].m_Stack[i]` (style unchecked, as the original)
fn big(style: u32, i: i32) -> GPtr<u8> {
    BIG_MESSAGES
        .ptr()
        .cast::<u8>()
        .byte_add(style as isize * 0xC0 + i as isize * 0x30)
}

/// `PreviousBriefs[i]`
fn prev(i: i32) -> GPtr<u8> {
    PREVIOUS_BRIEFS.ptr().cast::<u8>().byte_add(i as isize * 0x20)
}

fn cstr_len(p: GPtr<u8>) -> u32 {
    let mut n = 0u32;
    while rd::<u8>(p, n as isize) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

fn local(buf: &mut [u8]) -> GPtr<u8> {
    GPtr::from_raw(buf.as_mut_ptr())
}

/// `CMessages::GetStringLength(const GxtChar*)` (0x69DB50).
#[hook(addr = 0x69DB50, conv = cdecl, name = "CMessages::GetStringLength", expect = "8B 4C 24 04 8A 11")]
pub fn get_string_length(s: GPtr<u8>) -> u32 {
    cstr_len(s)
}

/// `CMessages::StringCopy(GxtChar* dst, const GxtChar* src, uint16 len)` (0x69DB70): at most `len - 1`
/// bytes then a NUL; a null `src` zero-fills `dst[0 .. len-1]` (inclusive).
#[hook(addr = 0x69DB70, conv = cdecl, name = "CMessages::StringCopy", expect = "0F B7 44 24 0C")]
pub fn string_copy(dst: GPtr<u8>, src: GPtr<u8>, len: u32) {
    let lim = i32::from(len as u16) - 1;
    let mut i: u16 = 0;
    if src.is_null() {
        while i32::from(i) < lim {
            wr::<u8>(dst, i as isize, 0);
            i = i.wrapping_add(1);
        }
    } else {
        while i32::from(i) < lim {
            let c = rd::<u8>(src, i as isize);
            if c == 0 {
                break;
            }
            wr::<u8>(dst, i as isize, c);
            i = i.wrapping_add(1);
        }
    }
    wr::<u8>(dst, i as isize, 0);
}

/// `CMessages::StringCompare(const GxtChar* a, const GxtChar* b, uint16 len)` (0x69DBD0): when the
/// lengths differ (low 16 bits), false if either is shorter than `len`; then the first `len` bytes of
/// `a` (up to its NUL) must match `b`.
#[hook(addr = 0x69DBD0, conv = cdecl, name = "CMessages::StringCompare", expect = "53 55 56 57 8B 7C 24 14")]
pub fn string_compare(a: GPtr<u8>, b: GPtr<u8>, len: u32) -> u8 {
    let len = len as u16;
    let la = cstr_len(a) as u16;
    let lb = cstr_len(b) as u16;
    if la != lb && (la < len || lb < len) {
        return 0;
    }
    if rd::<u8>(a, 0) == 0 {
        return 1;
    }
    let mut i: u16 = 0;
    loop {
        if i >= len {
            return 1;
        }
        if rd::<u8>(a, i as isize) != rd::<u8>(b, i as isize) {
            return 0;
        }
        i = i.wrapping_add(1);
        if rd::<u8>(a, i as isize) == 0 {
            return 1;
        }
    }
}

/// `CMessages::CutString(int16 width, GxtChar* str, GxtChar** lines)` (0x69DC50): splits `str` at the
/// space at or before every `width` bytes (no lower bound on the backwards scan), at most 8 lines.
#[hook(addr = 0x69DC50, conv = cdecl, name = "CMessages::CutString", expect = "51 53 55 56 8B 74 24 18")]
pub fn cut_string(width: i32, s: GPtr<u8>, lines: GPtr<GPtr<u8>>) -> i16 {
    let len = cstr_len(s) as i16;
    let width = width as i16;
    let mut n: i16 = 0;
    let mut pos: i16 = 0;
    if len <= 0 {
        return 0;
    }
    while n < 8 {
        lines.wrapping_index(i32::from(n)).write(s.byte_add(pos as isize));
        let end = i32::from(width) + i32::from(pos);
        n = n.wrapping_add(1);
        if end >= i32::from(len) {
            break;
        }
        pos = pos.wrapping_add(width);
        while rd::<u8>(s, pos as isize) != b' ' {
            pos = pos.wrapping_sub(1);
        }
        wr::<u8>(s, pos as isize, 0);
        pos = pos.wrapping_add(1);
        if pos >= len {
            break;
        }
    }
    n
}

fn clear_brief_entries() {
    for i in 0..8 {
        wr::<u32>(brief(i), 0, 0);
        wr::<u32>(brief(i), 0x28, 0);
    }
}

fn clear_previous_entries() {
    for i in 0..20 {
        wr::<u32>(prev(i), 0, 0);
        wr::<u32>(prev(i), 0x1C, 0);
    }
}

/// `CMessages::ClearMessages(bool ignoreMissionTitle)` (0x69DCD0): every big message style (with the flag
/// set, styles 1 and 4 are kept), then the briefs (text and string pointers only).
#[hook(addr = 0x69DCD0, conv = cdecl, name = "CMessages::ClearMessages", expect = "53 56 33 DB 33 D2")]
pub fn clear_messages(flag: u32) {
    for s in 0..7u32 {
        if flag as u8 != 0 && (s == 1 || s == 4) {
            continue;
        }
        for i in 0..4 {
            wr::<u32>(big(s, i), 0, 0);
            wr::<u32>(big(s, i), 0x28, 0);
        }
    }
    clear_brief_entries();
}

/// `CMessages::ClearSmallMessagesOnly()` (0x69DD30).
#[hook(addr = 0x69DD30, conv = cdecl, name = "CMessages::ClearSmallMessagesOnly", expect = "B8 18 A8 C1 00")]
pub fn clear_small_messages_only() {
    clear_brief_entries();
}

/// `CMessages::AddToPreviousBriefArray(text, n1..n6, string)` (0x69DD50): nothing when an identical
/// entry exists; otherwise the used entries shift down one (the 20th is dropped) and the new one goes
/// first. The scan reads one entry past the array (`BriefMessages[0].text`) when all 20 are used.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x69DD50, conv = cdecl, name = "CMessages::AddToPreviousBriefArray", expect = "A1 70 A5 C1 00")]
pub fn add_to_previous_brief_array(
    text: GPtr<u8>,
    n1: i32,
    n2: i32,
    n3: i32,
    n4: i32,
    n5: i32,
    n6: i32,
    s: GPtr<u8>,
) {
    let nums = [n1, n2, n3, n4, n5, n6];
    let mut i: u16 = 0;
    if !rd::<GPtr<u8>>(prev(0), 0).is_null() {
        loop {
            if i >= 20 {
                break;
            }
            let e = prev(i32::from(i));
            if (0..6isize).all(|k| rd::<i32>(e, 4 + 4 * k) == nums[k as usize])
                && rd::<GPtr<u8>>(e, 0) == text
                && rd::<GPtr<u8>>(e, 0x1C) == s
            {
                return;
            }
            i = i.wrapping_add(1);
            if rd::<GPtr<u8>>(prev(i32::from(i)), 0).is_null() {
                break;
            }
        }
        if i != 0 {
            let mut last = i.wrapping_sub(1);
            if i == 20 {
                last = last.wrapping_sub(1);
            }
            let mut k = last as i16;
            while k >= 0 {
                for w in 0..8 {
                    let v = rd::<u32>(prev(i32::from(k)), w * 4);
                    wr::<u32>(prev(i32::from(k) + 1), w * 4, v);
                }
                k -= 1;
            }
        }
    }
    let e = prev(0);
    wr(e, 0, text);
    for (k, n) in nums.iter().enumerate() {
        wr::<i32>(e, 4 + 4 * k as isize, *n);
    }
    wr(e, 0x1C, s);
}

/// `CMessages::ClearPreviousBriefArray()` (0x69DE70).
#[hook(addr = 0x69DE70, conv = cdecl, name = "CMessages::ClearPreviousBriefArray", expect = "B8 8C A5 C1 00")]
pub fn clear_previous_brief_array() {
    clear_previous_entries();
}

/// `CMessages::InsertNumberInString(str, n1..n6, dst)` (0x69DE90): every "~1~" becomes the next number
/// (`sprintf "%d"` + `AsciiToGxtChar`); after the sixth marker n6 is reused, the u8 marker counter wraps
/// (the 256th marker restarts at n2). A null `str` gives an empty `dst`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x69DE90, conv = cdecl, name = "CMessages::InsertNumberInString", expect = "83 EC 24 56 8B 74 24 2C")]
pub fn insert_number_in_string(
    s: GPtr<u8>,
    n1: i32,
    n2: i32,
    n3: i32,
    n4: i32,
    n5: i32,
    n6: i32,
    dst: GPtr<u8>,
) {
    if s.is_null() {
        wr::<u8>(dst, 0, 0);
        return;
    }
    let mut num = [0u8; 16];
    let mut gnum = [0u8; 16];
    let mut c: u8 = 0;
    sprintf1(local(&mut num), g(FMT_D), n1);
    let mut nlen = cstr_len(local(&mut num));
    ascii_to_gxt_char(local(&mut num), local(&mut gnum));
    let l = cstr_len(s);
    let mut out: u32 = 0;
    let mut i: u32 = 0;
    while i < l {
        let ch = rd::<u8>(s, i as isize);
        if ch == b'~' && rd::<u8>(s, i as isize + 1) == b'1' && rd::<u8>(s, i as isize + 2) == b'~' {
            i = i.wrapping_add(3);
            for k in 0..nlen {
                wr::<u8>(dst, out.wrapping_add(k) as isize, gnum[k as usize]);
            }
            out = out.wrapping_add(nlen);
            c = c.wrapping_add(1);
            let next = match c {
                1 => Some(n2),
                2 => Some(n3),
                3 => Some(n4),
                4 => Some(n5),
                5 => Some(n6),
                _ => None,
            };
            if let Some(n) = next {
                sprintf1(local(&mut num), g(FMT_D), n);
            }
            nlen = cstr_len(local(&mut num));
            ascii_to_gxt_char(local(&mut num), local(&mut gnum));
        } else {
            wr::<u8>(dst, out as isize, ch);
            out = out.wrapping_add(1);
            i = i.wrapping_add(1);
        }
    }
    wr::<u8>(dst, out as isize, 0);
}

/// `CMessages::InsertStringInString(GxtChar* target, const GxtChar* repl)` (0x69E040): every "~a~" (not
/// "~1~") becomes `repl`, building `strlen(target) + strlen(repl)` (u16) bytes (the scan reads past
/// `target`'s NUL when there is no marker); those bytes are copied back without a NUL and the rest of
/// `target` up to 400 bytes is zero-filled.
#[hook(addr = 0x69E040, conv = cdecl, name = "CMessages::InsertStringInString", expect = "81 EC 94 01 00 00")]
pub fn insert_string_in_string(target: GPtr<u8>, repl: GPtr<u8>) {
    if target.is_null() || repl.is_null() {
        return;
    }
    let mut buf = [0u8; 0x10000];
    let lt = cstr_len(target);
    let lr = cstr_len(repl);
    let total = lt.wrapping_add(lr) as u16;
    let mut inp: u16 = 0;
    let mut out: u16 = 0;
    while out < total {
        let p = target.byte_add(inp as isize);
        let ch = rd::<u8>(p, 0);
        if ch == b'~' && rd::<u8>(p, 1) == b'a' && rd::<u8>(p, 2) == b'~' {
            inp = inp.wrapping_add(3);
            for k in 0..(lr as u16) {
                buf[out as usize] = rd::<u8>(repl, k as isize);
                out = out.wrapping_add(1);
            }
        } else {
            buf[out as usize] = ch;
            out = out.wrapping_add(1);
            inp = inp.wrapping_add(1);
        }
    }
    buf[out as usize] = 0;
    for k in 0..total {
        wr::<u8>(target, k as isize, buf[k as usize]);
    }
    if total < 0x190 {
        for k in total..0x190 {
            wr::<u8>(target, k as isize, 0);
        }
    }
}

/// `CMessages::InsertPlayerControlKeysInString(GxtChar* str)` (0x69E160): every "~k~" skips one more
/// byte (the "~" of "~k~~NAME~") and, when one of the 59 action names matches (`StringCompare`) and is
/// followed by '~', becomes `GetGxtStringOfCommandKeys`; the result is copied back (strlen bytes) and
/// zero-filled to 400 bytes.
#[hook(addr = 0x69E160, conv = cdecl, name = "CMessages::InsertPlayerControlKeysInString", expect = "8B 54 24 04 81 EC 30 03 00 00")]
pub fn insert_player_control_keys_in_string(s: GPtr<u8>) {
    if s.is_null() {
        return;
    }
    let len = cstr_len(s) as u16;
    let mut keys = [0u8; 0x190];
    let mut out_buf = [0u8; 0x10000];
    let mut out: u16 = 0;
    let mut pos: u32 = 0;
    while (pos as u16) < len {
        let p = s.byte_add((pos as u16) as isize);
        let ch = rd::<u8>(p, 0);
        if ch == b'~' && rd::<u8>(p, 1) == b'k' && rd::<u8>(p, 2) == b'~' {
            pos = pos.wrapping_add(4);
            let mut found = false;
            let mut a: u16 = 0;
            while !found {
                let name = g::<u8>(ACTION_NAMES + usize::from(a) * 40);
                let nl = cstr_len(name);
                if nl as u16 != 0 {
                    let at = u32::from(pos as u16);
                    if string_compare_call(s.byte_add(at as isize), name, nl) != 0
                        && rd::<u8>(s, (at + u32::from(nl as u16)) as isize) == b'~'
                    {
                        found = true;
                        get_gxt_string_of_command_keys(
                            g(CONTROLS_MANAGER),
                            u32::from(a),
                            local(&mut keys),
                            0x190,
                        );
                        pos = nl.wrapping_add(pos).wrapping_add(1);
                        let kl = cstr_len(local(&mut keys)) as u16;
                        for k in 0..kl {
                            out_buf[out as usize] = keys[k as usize];
                            keys[k as usize] = 0;
                            out = out.wrapping_add(1);
                        }
                    }
                }
                a += 1;
                if a >= 0x3B {
                    break;
                }
            }
        } else {
            out_buf[out as usize] = ch;
            out = out.wrapping_add(1);
            pos = pos.wrapping_add(1);
        }
    }
    out_buf[out as usize] = 0;
    let l = cstr_len(local(&mut out_buf)) as u16;
    for k in 0..l {
        wr::<u8>(s, k as isize, out_buf[k as usize]);
    }
    if l < 0x190 {
        for k in l..0x190 {
            wr::<u8>(s, k as isize, 0);
        }
    }
}

/// Writes a `tMessage`.
fn set_message(
    m: GPtr<u8>,
    text: GPtr<u8>,
    flags: u16,
    time: u32,
    nums: [i32; 6],
    s: GPtr<u8>,
    prev_brief: Option<u8>,
) {
    wr(m, 0, text);
    wr::<u16>(m, 4, flags);
    wr::<u32>(m, 8, time);
    wr::<u32>(m, 0xC, TIME_MS.get());
    for (k, n) in nums.iter().enumerate() {
        wr::<i32>(m, 0x10 + 4 * k as isize, *n);
    }
    wr(m, 0x28, s);
    if let Some(p) = prev_brief {
        wr::<u8>(m, 0x2C, p);
    }
}

/// The first free brief slot (text null), or None.
fn free_brief() -> Option<i32> {
    (0..8).find(|i| rd::<GPtr<u8>>(brief(*i), 0).is_null())
}

/// The first free entry of big-message style `style`, or None.
fn free_big(style: u32) -> Option<i32> {
    (0..4).find(|i| rd::<GPtr<u8>>(big(style, *i), 0).is_null())
}

/// `AddToPreviousBriefArray` with `BriefMessages[0]`'s fields.
fn brief0_to_previous() {
    let m = brief(0);
    let n = |k: isize| rd::<i32>(m, 0x10 + 4 * k);
    add_to_previous_brief_array_call(rd(m, 0), n(0), n(1), n(2), n(3), n(4), n(5), rd(m, 0x28));
}

/// The dead local formatting every `Add*` does before storing (result discarded): `StringCopy` (or
/// `InsertNumberInString`), optionally `InsertStringInString`, then `InsertPlayerControlKeysInString`.
fn scratch_format(text: GPtr<u8>, nums: Option<[i32; 6]>, s: Option<GPtr<u8>>) {
    let mut buf = [0u8; 0x320];
    let b = local(&mut buf);
    match nums {
        Some(n) => insert_number_in_string_call(text, n[0], n[1], n[2], n[3], n[4], n[5], b),
        None => string_copy_call(b, text, 0x190),
    }
    if let Some(s) = s {
        insert_string_in_string_call(b, s);
    }
    insert_player_control_keys_in_string_call(b);
}

const NO_NUMS: [i32; 6] = [-1; 6];

/// `CMessages::AddMessageWithNumberQ(text, time, flags, n1..n6, bPreviousBrief)` (0x69E360).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x69E360, conv = cdecl, name = "CMessages::AddMessageWithNumberQ", expect = "8B 4C 24 14 8B 54 24 10")]
pub fn add_message_with_number_q(
    text: GPtr<u8>,
    time: u32,
    flags: u32,
    n1: i32,
    n2: i32,
    n3: i32,
    n4: i32,
    n5: i32,
    n6: i32,
    prev_brief: u32,
) {
    let nums = [n1, n2, n3, n4, n5, n6];
    scratch_format(text, Some(nums), None);
    let Some(i) = free_brief() else { return };
    set_message(
        brief(i),
        text,
        flags as u16,
        time,
        nums,
        GPtr::NULL,
        Some(prev_brief as u8),
    );
    if i == 0 && prev_brief as u8 != 0 {
        brief0_to_previous();
    }
}

/// `CMessages::AddMessageJumpQWithNumber(text, time, flags, n1..n6, bPreviousBrief)` (0x69E4E0).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x69E4E0, conv = cdecl, name = "CMessages::AddMessageJumpQWithNumber", expect = "8B 4C 24 14 8B 54 24 10")]
pub fn add_message_jump_q_with_number(
    text: GPtr<u8>,
    time: u32,
    flags: u32,
    n1: i32,
    n2: i32,
    n3: i32,
    n4: i32,
    n5: i32,
    n6: i32,
    prev_brief: u32,
) {
    let nums = [n1, n2, n3, n4, n5, n6];
    scratch_format(text, Some(nums), None);
    set_message(
        brief(0),
        text,
        flags as u16,
        time,
        nums,
        GPtr::NULL,
        Some(prev_brief as u8),
    );
    if prev_brief as u8 != 0 {
        add_to_previous_brief_array_call(text, n1, n2, n3, n4, n5, n6, GPtr::NULL);
    }
}

/// `CMessages::AddBigMessageWithNumber(text, time, style, n1..n6)` (0x69E5F0): entry 0 of the style
/// (flags 0, string null, previous-brief byte untouched).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x69E5F0, conv = cdecl, name = "CMessages::AddBigMessageWithNumber", expect = "8B 4C 24 14 8B 54 24 10")]
pub fn add_big_message_with_number(
    text: GPtr<u8>,
    time: u32,
    style: u32,
    n1: i32,
    n2: i32,
    n3: i32,
    n4: i32,
    n5: i32,
    n6: i32,
) {
    let nums = [n1, n2, n3, n4, n5, n6];
    scratch_format(text, Some(nums), None);
    set_message(
        big(u32::from(style as u16), 0),
        text,
        0,
        time,
        nums,
        GPtr::NULL,
        None,
    );
}

/// `CMessages::AddBigMessageWithNumberQ(text, time, style, n1..n6)` (0x69E6E0): the first free entry.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x69E6E0, conv = cdecl, name = "CMessages::AddBigMessageWithNumberQ", expect = "8B 4C 24 14 8B 54 24 10")]
pub fn add_big_message_with_number_q(
    text: GPtr<u8>,
    time: u32,
    style: u32,
    n1: i32,
    n2: i32,
    n3: i32,
    n4: i32,
    n5: i32,
    n6: i32,
) {
    let nums = [n1, n2, n3, n4, n5, n6];
    scratch_format(text, Some(nums), None);
    let style = u32::from(style as u16);
    let Some(i) = free_big(style) else { return };
    set_message(big(style, i), text, 0, time, nums, GPtr::NULL, None);
}

/// `CMessages::AddMessageWithStringQ(text, time, flags, string, bPreviousBrief)` (0x69E800).
#[hook(addr = 0x69E800, conv = cdecl, name = "CMessages::AddMessageWithStringQ", expect = "81 EC 20 03 00 00")]
pub fn add_message_with_string_q(text: GPtr<u8>, time: u32, flags: u32, s: GPtr<u8>, prev_brief: u32) {
    scratch_format(text, None, Some(s));
    let Some(i) = free_brief() else { return };
    set_message(
        brief(i),
        text,
        flags as u16,
        time,
        NO_NUMS,
        s,
        Some(prev_brief as u8),
    );
    if i == 0 && prev_brief as u8 != 0 {
        brief0_to_previous();
    }
}

/// `CMessages::AddMessageJumpQWithString(text, time, flags, string, bPreviousBrief)` (0x69E950).
#[hook(addr = 0x69E950, conv = cdecl, name = "CMessages::AddMessageJumpQWithString", expect = "81 EC 20 03 00 00")]
pub fn add_message_jump_q_with_string(text: GPtr<u8>, time: u32, flags: u32, s: GPtr<u8>, prev_brief: u32) {
    scratch_format(text, None, Some(s));
    set_message(
        brief(0),
        text,
        flags as u16,
        time,
        NO_NUMS,
        s,
        Some(prev_brief as u8),
    );
    if prev_brief as u8 != 0 {
        add_to_previous_brief_array_call(text, -1, -1, -1, -1, -1, -1, s);
    }
}

/// `CMessages::ClearAllMessagesDisplayedByGame(bool)` (0x69EDC0): `ClearMessages(flag)`, the previous
/// briefs (inlined), `CHud::GetRidOfAllHudMessages(flag)`.
#[hook(addr = 0x69EDC0, conv = cdecl, name = "CMessages::ClearAllMessagesDisplayedByGame", expect = "56 8B 74 24 08")]
pub fn clear_all_messages_displayed_by_game(flag: u32) {
    clear_messages_call(flag);
    clear_previous_entries();
    hud_get_rid_of_all_hud_messages(flag);
}

/// `CMessages::Init()` (0x69EE00): big messages, briefs, previous briefs (text and string pointers).
#[hook(addr = 0x69EE00, conv = cdecl, name = "CMessages::Init", expect = "56 B8 98 A9 C1 00")]
pub fn init() {
    for s in 0..7 {
        for i in 0..4 {
            wr::<u32>(big(s, i), 0, 0);
            wr::<u32>(big(s, i), 0x28, 0);
        }
    }
    clear_brief_entries();
    clear_previous_entries();
}

/// Expires entry 0 of a queue of `n` messages at `first` (stride 0x30): shifts the following used entries
/// down (stopping at the first free one, at most `n - 1` copies) and clears the last copied slot's text.
fn pop_queue(first: GPtr<u8>, n: i32) {
    wr::<u32>(first, 0, 0);
    let mut k = 0i32;
    loop {
        let next = first.byte_add((k as isize + 1) * 0x30);
        if rd::<GPtr<u8>>(next, 0).is_null() {
            break;
        }
        k += 1;
        for w in 0..12 {
            let v = rd::<u32>(next, w * 4);
            wr::<u32>(first.byte_add((k as isize - 1) * 0x30), w * 4, v);
        }
        if k >= n - 1 {
            break;
        }
    }
    wr::<u32>(first.byte_add(k as isize * 0x30), 0, 0);
}

/// `CMessages::Process()` (0x69EE60): expires the first entry of each big style and of the briefs once
/// `time + start < now` (unsigned), shifting the queue and restarting the new first entry's clock; a new
/// first brief flagged for the history goes to `AddToPreviousBriefArray`.
#[hook(addr = 0x69EE60, conv = cdecl, name = "CMessages::Process", expect = "51 53 55 56 33 D2")]
pub fn process() {
    for s in 0..7 {
        let m = big(s, 0);
        if rd::<u32>(m, 0) == 0 {
            continue;
        }
        if TIME_MS.get() <= rd::<u32>(m, 8).wrapping_add(rd::<u32>(m, 0xC)) {
            continue;
        }
        pop_queue(m, 4);
        wr::<u32>(m, 0xC, TIME_MS.get());
    }
    let m = brief(0);
    if rd::<u32>(m, 0) == 0 {
        return;
    }
    let now = TIME_MS.get();
    if now <= rd::<u32>(m, 8).wrapping_add(rd::<u32>(m, 0xC)) {
        return;
    }
    pop_queue(m, 8);
    wr::<u32>(m, 0xC, now);
    if rd::<u32>(m, 0) != 0 && rd::<u8>(m, 0x2C) != 0 {
        brief0_to_previous();
    }
}

/// `CMessages::Display(bool)` (0x69EFC0): with the flag set, every big style's first entry is formatted
/// (numbers, string, keys) into `CHud::SetBigMessage(buf, style)`; when the flag equals the byte at
/// 0xA44488 the first brief goes to `CHud::SetMessage`.
#[hook(addr = 0x69EFC0, conv = cdecl, name = "CMessages::Display", expect = "81 EC 90 01 00 00")]
pub fn display(flag: u32) {
    let flag = flag as u8;
    let mut buf = [0u8; 0x400];
    let format = |m: GPtr<u8>, b: GPtr<u8>| {
        let n = |k: isize| rd::<i32>(m, 0x10 + 4 * k);
        insert_number_in_string_call(rd(m, 0), n(0), n(1), n(2), n(3), n(4), n(5), b);
        insert_string_in_string_call(b, rd(m, 0x28));
        insert_player_control_keys_in_string_call(b);
    };
    if flag != 0 {
        for s in 0..7u32 {
            let b = local(&mut buf);
            format(big(s, 0), b);
            hud_set_big_message(b, s);
        }
    }
    if flag == g::<u8>(DISPLAY_BRIEF_FLAG).read() {
        let b = local(&mut buf);
        format(brief(0), b);
        hud_set_message(b);
    }
}

/// `CMessages::AddMessageQ(text, time, flags, bPreviousBrief)` (0x69F0B0).
#[hook(addr = 0x69F0B0, conv = cdecl, name = "CMessages::AddMessageQ", expect = "81 EC 20 03 00 00")]
pub fn add_message_q(text: GPtr<u8>, time: u32, flags: u32, prev_brief: u32) {
    scratch_format(text, None, None);
    let Some(i) = free_brief() else { return };
    set_message(
        brief(i),
        text,
        flags as u16,
        time,
        NO_NUMS,
        GPtr::NULL,
        Some(prev_brief as u8),
    );
    if i == 0 && prev_brief as u8 != 0 {
        brief0_to_previous();
    }
}

/// `CMessages::AddMessageJump(text, time, flags, bPreviousBrief)` (0x69F1E0).
#[hook(addr = 0x69F1E0, conv = cdecl, name = "CMessages::AddMessageJump", expect = "81 EC 20 03 00 00")]
pub fn add_message_jump(text: GPtr<u8>, time: u32, flags: u32, prev_brief: u32) {
    scratch_format(text, None, None);
    set_message(
        brief(0),
        text,
        flags as u16,
        time,
        NO_NUMS,
        GPtr::NULL,
        Some(prev_brief as u8),
    );
    if prev_brief as u8 != 0 {
        add_to_previous_brief_array_call(text, -1, -1, -1, -1, -1, -1, GPtr::NULL);
    }
}

/// `CMessages::AddBigMessage(text, time, style)` (0x69F2B0): entry 0 of the style.
#[hook(addr = 0x69F2B0, conv = cdecl, name = "CMessages::AddBigMessage", expect = "81 EC 20 03 00 00")]
pub fn add_big_message(text: GPtr<u8>, time: u32, style: u32) {
    scratch_format(text, None, None);
    set_message(
        big(u32::from(style as u16), 0),
        text,
        0,
        time,
        NO_NUMS,
        GPtr::NULL,
        None,
    );
}

/// `CMessages::AddBigMessageQ(text, time, style)` (0x69F370): the first free entry of the style.
#[hook(addr = 0x69F370, conv = cdecl, name = "CMessages::AddBigMessageQ", expect = "81 EC 20 03 00 00")]
pub fn add_big_message_q(text: GPtr<u8>, time: u32, style: u32) {
    scratch_format(text, None, None);
    let style = u32::from(style as u16);
    let Some(i) = free_big(style) else { return };
    set_message(big(style, i), text, 0, time, NO_NUMS, GPtr::NULL, None);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_STRING_LENGTH,
    HOOK_STRING_COPY,
    HOOK_STRING_COMPARE,
    HOOK_CUT_STRING,
    HOOK_CLEAR_MESSAGES,
    HOOK_CLEAR_SMALL_MESSAGES_ONLY,
    HOOK_ADD_TO_PREVIOUS_BRIEF_ARRAY,
    HOOK_CLEAR_PREVIOUS_BRIEF_ARRAY,
    HOOK_INSERT_NUMBER_IN_STRING,
    HOOK_INSERT_STRING_IN_STRING,
    HOOK_INSERT_PLAYER_CONTROL_KEYS_IN_STRING,
    HOOK_ADD_MESSAGE_WITH_NUMBER_Q,
    HOOK_ADD_MESSAGE_JUMP_Q_WITH_NUMBER,
    HOOK_ADD_BIG_MESSAGE_WITH_NUMBER,
    HOOK_ADD_BIG_MESSAGE_WITH_NUMBER_Q,
    HOOK_ADD_MESSAGE_WITH_STRING_Q,
    HOOK_ADD_MESSAGE_JUMP_Q_WITH_STRING,
    HOOK_CLEAR_ALL_MESSAGES_DISPLAYED_BY_GAME,
    HOOK_INIT,
    HOOK_PROCESS,
    HOOK_DISPLAY,
    HOOK_ADD_MESSAGE_Q,
    HOOK_ADD_MESSAGE_JUMP,
    HOOK_ADD_BIG_MESSAGE,
    HOOK_ADD_BIG_MESSAGE_Q,
];
