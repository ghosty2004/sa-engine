//! `CClock::Initialise`, `GetGameClockMinutesUntil`, `StoreClock`, `RestoreClock`, `OffsetClockByADay`
//! (STD-6, batch `std6-render-1`). Ported from the 1.0 US bytes (0x52CD90..0x52D145).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::{GPtr, GameData};

fn g<T: GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

const MS_PER_MINUTE: usize = 0xB7015C;
const LAST_TICK: usize = 0xB70158;
const MONTH: usize = 0xB70155;
const DAY: usize = 0xB70154;
const HOURS: usize = 0xB70153;
const MINUTES: usize = 0xB70152;
const SECONDS: usize = 0xB70150;
const STORED: usize = 0xB70144;
const WEEKDAY: usize = 0xB7014E;
/// `CTimer::m_snTimeInMilliseconds`; the days per month (.data, 1-based by month).
const TIME_MS: usize = 0xB7CB84;
const DAYS_IN_MONTH: usize = 0x8CCF23;

/// `CClock::Initialise(msPerMinute)` (0x52CD90): 1 January, 12:00:00, last tick = now, nothing stored,
/// weekday 4.
#[hook(addr = 0x52CD90, conv = cdecl, name = "CClock::Initialise", expect = "8B 44 24 04 8B 0D 84 CB B7 00",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn clock_initialise(ms_per_minute: u32) {
    let now = g::<u32>(TIME_MS).read();
    g::<u32>(MS_PER_MINUTE).write(ms_per_minute);
    g::<u8>(MONTH).write(1);
    g::<u8>(DAY).write(1);
    g::<u32>(LAST_TICK).write(now);
    g::<u8>(HOURS).write(0xC);
    g::<u8>(MINUTES).write(0);
    g::<u16>(SECONDS).write(0);
    g::<u8>(STORED).write(0);
    g::<u8>(WEEKDAY).write(4);
}

/// `CClock::GetGameClockMinutesUntil(hours, minutes)` (0x52CEB0): minutes from now until hh:mm,
/// wrapping over midnight (an earlier time adds 1440).
#[hook(addr = 0x52CEB0, conv = cdecl, name = "CClock::GetGameClockMinutesUntil", expect = "0F B6 0D 53 01 B7 00",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn get_game_clock_minutes_until(hours: u32, minutes: u32) -> u32 {
    let now = g::<u8>(HOURS).read() as u32 * 60 + g::<u8>(MINUTES).read() as u32;
    let mut t = (hours & 0xFF) * 60 + (minutes & 0xFF);
    if t < now {
        t += 0x5A0;
    }
    t.wrapping_sub(now)
}

/// `CClock::StoreClock()` (0x52D020): month, day, hours, minutes, seconds saved (0xB7014D..0xB70148),
/// stored flag set.
#[hook(addr = 0x52D020, conv = cdecl, name = "CClock::StoreClock", expect = "A0 55 01 B7 00 8A 0D 54 01 B7 00",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn store_clock() {
    g::<u8>(0xB7014D).write(g::<u8>(MONTH).read());
    g::<u8>(0xB7014C).write(g::<u8>(DAY).read());
    g::<u8>(0xB7014B).write(g::<u8>(HOURS).read());
    g::<u8>(0xB7014A).write(g::<u8>(MINUTES).read());
    g::<u16>(0xB70148).write(g::<u16>(SECONDS).read());
    g::<u8>(STORED).write(1);
}

/// `CClock::RestoreClock()` (0x52D070): the saved values back.
#[hook(addr = 0x52D070, conv = cdecl, name = "CClock::RestoreClock", expect = "A0 4D 01 B7 00 8A 0D 4C 01 B7 00",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn restore_clock() {
    g::<u8>(MONTH).write(g::<u8>(0xB7014D).read());
    g::<u8>(DAY).write(g::<u8>(0xB7014C).read());
    g::<u8>(HOURS).write(g::<u8>(0xB7014B).read());
    g::<u8>(MINUTES).write(g::<u8>(0xB7014A).read());
    g::<u16>(SECONDS).write(g::<u16>(0xB70148).read());
}

/// `CClock::OffsetClockByADay(forward)` (0x52D0B0): one day forward (day past the month's length ->
/// day 1 of the next month, month past 12 -> 1; weekday past 7 -> 1) or back (day 0 -> the previous
/// month's last day, month 0 -> 12; weekday 0 -> 7).
#[hook(addr = 0x52D0B0, conv = cdecl, name = "CClock::OffsetClockByADay", expect = "8B 44 24 04 85 C0 74 54",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn offset_clock_by_a_day(forward: u32) {
    if forward != 0 {
        let month = g::<u8>(MONTH).read();
        let day = g::<u8>(DAY).read().wrapping_add(1);
        g::<u8>(DAY).write(day);
        if day > g::<u8>(DAYS_IN_MONTH + month as usize).read() {
            g::<u8>(DAY).write(1);
            let m = month.wrapping_add(1);
            g::<u8>(MONTH).write(m);
            if m > 0xC {
                g::<u8>(MONTH).write(1);
            }
        }
        let wd = g::<u8>(WEEKDAY).read().wrapping_add(1);
        g::<u8>(WEEKDAY).write(wd);
        if wd > 7 {
            g::<u8>(WEEKDAY).write(1);
        }
    } else {
        let day = g::<u8>(DAY).read().wrapping_sub(1);
        g::<u8>(DAY).write(day);
        if day == 0 {
            let mut m = g::<u8>(MONTH).read().wrapping_sub(1);
            g::<u8>(MONTH).write(m);
            if m == 0 {
                m = 0xC;
                g::<u8>(MONTH).write(m);
            }
            g::<u8>(DAY).write(g::<u8>(DAYS_IN_MONTH + m as usize).read());
        }
        let wd = g::<u8>(WEEKDAY).read().wrapping_sub(1);
        g::<u8>(WEEKDAY).write(wd);
        if wd == 0 {
            g::<u8>(WEEKDAY).write(7);
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CLOCK_INITIALISE,
    HOOK_GET_GAME_CLOCK_MINUTES_UNTIL,
    HOOK_STORE_CLOCK,
    HOOK_RESTORE_CLOCK,
    HOOK_OFFSET_CLOCK_BY_A_DAY,
];
