# COnscreenTimer (HUD) - full specs

Layout (gr OnscreenTimer.h, VALIDATE_SIZE 0x154, confirmed by the offsets below): +0x00 m_Clock (COnscreenTimerEntry, 0x40:
+0 m_nVarId u32, +4 m_szDescriptionTextKey[10], +0xE m_szDisplayedText[42], +0x38 m_bEnabled, +0x39 m_nTimerDirection u8
(0 INCREASE, 1 DECREASE), +0x3C m_nClockBeepCountdownSecs u32); +0x40 m_aCounters[4] (COnscreenCounterEntry, 0x44 each:
+0 m_nVarId, +4 m_nMaxVarValue, +8 m_szDescriptionTextKey[10], +0x12 m_nType u16 (0 SIMPLE, 1 LINE, 2 DOUBLE),
+0x14 m_szDisplayedText[42], +0x3E m_bEnabled, +0x3F m_bFlashWhenFirstDisplayed, +0x40 m_nColourId); +0x150 m_bDisplay;
+0x151 m_bPaused. Var ids are byte offsets into `CTheScripts::ScriptSpace` 0xA49960 (value = i32 at ScriptSpace + id).
Single instance: `CUserDisplay::OnscnTimer`.

## 0x44CBC0 COnscreenTimer::Init  (catalogue: specified/REA, port level 0)
- Entry `jmp 0x15701D0` (.HOODLUM body). `__thiscall void ()`, plain `ret`. Caller: CUserDisplay::Init 0x571EE0.
- Behaviour in order: `m_bPaused (+0x151) = 0` (0x15701D5). For each of the 4 counters (0x15701E3-0x1570206, ascending):
  `m_nVarId = 0`, `m_nMaxVarValue = 0`, key bytes +8..+0x11 = 0 (dword, dword, word), `m_nType (u16) = 0`,
  `m_bEnabled = 0`, `m_bFlashWhenFirstDisplayed = 1` (0x15701FE `mov byte [eax+0x3b], 1`). m_szDisplayedText and
  m_nColourId are NOT touched. Then the clock: `m_nVarId = 0`, key bytes +4..+0xD = 0, `m_bEnabled (+0x38) = 0`,
  `m_nTimerDirection (+0x39) = 1` (DECREASE), `m_nClockBeepCountdownSecs (+0x3C) = 12` (0x157021F). m_szDisplayedText
  and m_bDisplay are not touched.
- gta-reversed (OnscreenTimer.cpp:20, not hooked): counter Init (OnscreenCounterEntry.cpp:13) sets
  `m_bFlashWhenFirstDisplayed = false`, `m_nColourId = HUD_COLOUR_RED` and memsets m_szDisplayedText - all three differ from
  the original (flash = 1, colour and text untouched). Port the original. Order counters-then-clock matches gr.
- Open questions: none.

## 0x44CC20 COnscreenTimer::ProcessForDisplay  (catalogue: specified/REA, port level 0)
- Entry `jmp 0x1569FD0` (.HOODLUM body). `__thiscall void ()`, plain `ret`. Caller: Game::Render2dStuff 0x53E230.
- Callee: CRT `sprintf` 0x821BB5 (cdecl, caller pops). Formats: 0x859A6C "%02d:%02d", 0x859508 "%d", 0x859A78 "%d / %d".
- Behaviour:
  1. If `CHud::m_Wants_To_Draw_Hud` (byte 0xBAA45D) == 0 return (0x1569FDB).
  2. `m_bDisplay = 0`, `m_Clock.m_bEnabled = 0`. If `m_Clock.m_nVarId != 0` (unsigned test, 0x1569FF0 `jbe`):
     `t = *(i32*)(ScriptSpace + varId)`; `q = t / 1000` (signed, truncation toward zero: `imul` by a magic constant,
     `sar edx, 6`, add sign bit, 0x156A013-0x156A01D); `sprintf(m_Clock.m_szDisplayedText (+0xE), "%02d:%02d",
     (q / 60) % 100, q % 60)` (both C signed div/mod, 0x156A027/0x156A030); `m_Clock.m_bEnabled = 1`, `m_bDisplay = 1`.
  3. For each counter c (index 0..3, ascending, 0x156A077-0x156A0F1): `c.m_bEnabled = 0`; if `c.m_nVarId == 0` continue.
     `t2 = *(i32*)(ScriptSpace + c.m_nVarId)`; `t1 = t2 < 0 ? 0 : t2` (setl/dec/and, 0x156A08E-0x156A094).
     type = `c.m_nType` (u16): 0 or 1 -> `sprintf(c.m_szDisplayedText, "%d", t1)`; 2 -> `max = *(i32*)(ScriptSpace +
     c.m_nMaxVarValue)` (0x156A09C-0x156A09F) and `sprintf(c.m_szDisplayedText, "%d / %d", t1, max)`; other values ->
     no sprintf. In all non-zero-var cases `c.m_bEnabled = 1`, `m_bDisplay = 1` (0x156A0D7/0x156A0DB).
- Obfuscation: the division magic is not an immediate. 0x1569FFA-0x156A012 is `pushfd; eax = 0x24402B1C ^ 0x248B58FC
  (= 0x00CB73E0); push esp; push 0x155FFE0; call eax; popfd`, i.e. the protection VM 0xCB73E0 returns the constant in EAX.
  The format pointer for the clock is computed as `rol([0x1560558] = 0x360042CD, [0x45BA91] & 31 = 9) = 0x859A6C`
  (0x156A036-0x156A049; static data, decodes to "%02d:%02d").
- gta-reversed: OnscreenTimer.cpp:147 + ProcessForDisplayClock / ProcessForDisplayCounter. Clock part matches. Counter
  DOUBLE: gr (OnscreenCounterEntry.cpp:40) prints `t1, t2` (the same variable twice, second unclamped); the original
  prints the variable at `m_nMaxVarValue` (also confirmed in the standalone 0x44CA90 at 0x44CAAE-0x44CAB1). gr bug; port
  the original.
- Open questions: 0x156A010 - the value returned by the VM call is not visible in the bytes; the `imul; sar edx, 6;
  shr eax, 31; add` sequence is the MSVC idiom for signed `/ 1000` with magic 0x10624DD3, and gr uses `/ 1000`. Treat as
  `/ 1000` (confidence high, but not byte-proven).

## 0x44CD30 COnscreenTimer::Process  (catalogue: specified/REA, port level 8)
- `__thiscall void ()`, plain `ret` / tail jump. Caller: CUserDisplay::Process 0x5720A0.
- If `CReplay::Mode` (byte 0xA43088) == 1 (playback) return; if `m_bPaused (+0x151)` != 0 return; else tail-jump
  `COnscreenTimerEntry::Process 0x44CB10` with ECX = this (= &m_Clock) (0x44CD43). Counters are not processed.
  gr OnscreenTimer.cpp:139 matches. Open questions: none.
