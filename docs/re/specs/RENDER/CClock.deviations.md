# CClock (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/Clock.cpp` (pin 20d8d2a). Globals: ms_nGameClockSeconds 0xB70150 (u16), ms_nGameClockMinutes
0xB70152, ms_nGameClockHours 0xB70153, ms_nGameClockDays 0xB70154, ms_nGameClockMonth 0xB70155, CurrentDay 0xB7014E (all u8),
ms_nLastClockTick 0xB70158, ms_nMillisecondsPerGameMinute 0xB7015C (u32), daysInMonth 0x8CCF24 (u8[12]),
CCheat::m_aCheatsActive 0x969130 (bool[]; [11] = CHEAT_FASTER_CLOCK 0x96913B, [55] = ALWAYS_MIDNIGHT 0x969167, [56] =
STOP_GAME_CLOCK_ORANGE_SKY 0x969168).

## 0x52CF10 CClock::Update  (gr_deviations = 1)
Original: `__cdecl void ()`, plain `ret` (0x52D01F).
Row Clock.cpp:46 (`if (gbFreezeTime) { // NOTSA ... ms_nLastClockTick = now; }`).
- Original bytes: no freeze-time branch. Sequence: `d = now - ms_nLastClockTick` (0x52CF1E); if `d > msPerMin` (SIGNED `jg`, 0x52CF31)
  or FASTER_CLOCK cheat (0x52CF33): if neither cheat 55 (0x52CF3B) nor 56 (0x52CF47): `++minutes` (0x52CF53),
  `ms_nLastClockTick += msPerMin` (0x52CF5F, msPerMin read once at 0x52CF22), and if FASTER_CLOCK `ms_nLastClockTick = now` (0x52CF6B,
  `now` read once at 0x52CF18); `minutes >= 60` (unsigned, 0x52CF71) -> minutes = 0, `++hours`; `hours >= 24` -> hours = 0, `++days`,
  `CurrentDay = (CurrentDay == 7) ? 1 : CurrentDay + 1` (0x52CFA7-0x52CFBA), `CStats::IncrementStat(0x86, 1.0f)` (0x52CFCA);
  `days >= daysInMonth[month - 1]` (byte table at 0x8CCF23 + month, 0x52CFE0, unsigned) -> days = 1, `++month`, `month > 12` -> 1.
  Always: `ms_nGameClockSeconds = (u16)(((now2 - ms_nLastClockTick) * 60) / msPerMin)` (0x52D001-0x52D019; now2 re-read, unsigned `div`).
- Verdict: CONFIRMED - gr adds a debug-only freeze branch that the original does not have. With gbFreezeTime = false (its only value
  outside the gr debug menu) the paths coincide. Class: harmless (port omits the branch).
Found in passing (CONFIRMED, keep-original): the elapsed-time test is a SIGNED compare in the original (`cmp eax, esi; jg` 0x52CF2A/
0x52CF31: `(int32)(now - last) > (int32)msPerMin`), while gr's `ms_nMillisecondsPerGameMinute < (GetTimeInMS() - ms_nLastClockTick)`
is unsigned (both uint32). They differ when `now < ms_nLastClockTick` (e.g. after a timer reset or a loaded save with a later tick): the
original does not advance the clock, gr advances one minute per frame.
