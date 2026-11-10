# CTaskComplexUseAttractor: gta-reversed deviation notes (AI)

Source: `re/gta-reversed/source/game_sa` (pin 20d8d2a). Verdicts: CONFIRMED = gr differs from the bytes (port follows the bytes), REFUTED = no behavioural difference, UNCLEAR = open question. Written by RE-SPEC-3 group I4.

## 0x6385D0 CTaskComplexUseAttractor::CreateFirstSubTask  (gr_deviations = 4)
Original bytes (`__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`, SEH; +0xC m_attractor; ped flags dword2 = +0x474):
  1. `g_InterestingEvents.Add(3, ped)` 0x602590 (0x6385ED-0x6385F7) - FIRST, unconditionally.
  2. `type = m_attractor->vtbl[0]()` (GetType); if type > 9 -> step 4 with result 0. Jump table 0x6388D0:
     0 ATM -> `new(0x20) CTaskSimpleUseAtm` 0x48DFE0;
     1 SEAT and 9 STEP (both 0x63863F) -> `r = GetRandomNumberInRange(0, 100)` 0x407180; `time = r > 40 ? 9999999 : GetRandomNumberInRange(15000,
       60000)` (0x638643-0x638669); `new(0x14)`; if ok `CTaskComplexSitDownThenIdleThenStandUp(t, time, m_attractor->GetType() == 9 (second
       virtual call), (ped[+0x474] >> 21) & 1 = bUseAttractorInstantly)` 0x631460 (the flag is pushed before the GetType call, 0x63868B-0x6386AC);
     2 STOP -> `new(0x10) CTaskComplexWaitForBus` 0x632900;
     3 PIZZA -> `t = GetRandomNumberInRange(6000, 12000)` (BEFORE the allocation); `new(0x20) CTaskSimpleStandStill(t, 0, 0, 8.0f)` 0x62F310;
     4 SHELTER -> `new(0xC) CTaskComplexWaitForDryWeather` 0x632800;
     5 TRIGGER_SCRIPT (0x63874F): if `ped[+0x474] & 0x800000` (bHasAScriptBrain) -> 0; if `!ScriptsForBrains.HasAttractorScriptBrainWithThisNameLoaded
       (attractor+0x84)` -> 0; `e = attractor->m_Entity (+8)`; if e == 0 -> 0; if e is NOT an object (`(e+0x36 & 7) != 4`) OR object flag
       `byte e+0x143 & 1` set: `ScriptsForBrains.StartAttractorScriptBrainWithThisName(name, ped, bUseAttractorInstantly)` 0x46B390; THEN in both
       cases `CScriptedBrainTaskStore::Clear(ped)` 0x635850 and `new(0x1C) CTaskComplexUseScriptedBrain(name)` 0x6358B0 (0x6387BE-0x6387EE);
     6 LOOK_AT -> `t = GetRandomNumberInRange(3000, 6000)`; `new(0x20) CTaskSimpleStandStill(t, 0, 0, 8.0f)`;
     7 SCRIPTED -> `new(0x10) CTaskComplexUseScriptedAttractor(m_attractor)` 0x633330;
     8 PARK -> `new(0x38) CTaskComplexSunbathe(0, 1)` 0x631F80.
  3. Allocation failure gives 0.
  4. If the result is 0: `CPedAttractorManager::BroadcastDeparture(GetPedAttractorManager(), ped, m_attractor)` 0x5EC980 (0x638894-0x6388A0).
  5. ALWAYS `ped[+0x474] &= ~0x200000` (clear bUseAttractorInstantly, 0x6388A5-0x6388B4); return the result.
Row TaskComplexUseAttractor.cpp:66 (`RandomBool(40.f) ? GetRandomNumberInRange(15000, 60000) : gDefaultTaskTime`):
- Original: `GetRandomNumberInRange(0,100) 0x407180 > 40` selects 9999999, otherwise the range (r in 0..40 -> 41% range); both via the
  original helper. gr's RandomBool and helper use different formulas/thresholds.
- Verdict: CONFIRMED. Class: keep-original.
Rows :73 (`StandStill{GetRandomNumberInRange(6000, 12000)}`) and :96 (`(3000, 6000)`):
- Original: 0x407180 at 0x6386EA / 0x638802 (drawn before `operator new`). Verdict: CONFIRMED (helper formula). Class: keep-original.
Row :102 (`default: NOTSA_UNREACHABLE()`):
- Original: type > 9 -> BroadcastDeparture, return 0 (0x638608 -> 0x638894), flag cleared.
- Verdict: CONFIRMED. Class: keep-original.
Additional CONFIRMED differences (no rows; gr is wrong):
- gr never calls `g_InterestingEvents.Add(3, ped)` (step 1) and never clears bUseAttractorInstantly (step 5).
- SEAT/STEP: gr passes `ped->bStayInSamePlace` (+0x46C bit 21); the original passes bUseAttractorInstantly (+0x474 bit 21).
- TRIGGER_SCRIPT on an object without flag +0x143 bit0: gr returns nullptr (and broadcasts departure); the original skips only the
  Start call and still returns a new CTaskComplexUseScriptedBrain (0x638794-0x63879B -> 0x6387BE).
