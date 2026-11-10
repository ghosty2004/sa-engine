# CTaskComplexGangLeader (AI) - gta-reversed deviation notes, RE-SPEC-3 group I3

Source: `Tasks/TaskTypes/TaskComplexGangLeader.cpp` (pin 20d8d2a). RNG: original int helper 0x407180 = `min + _ftol((rand() & 0xFFFF) * 3.0517578e-05f * (max - min))`;
inlined percent rolls are `_ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * 100.0f [0x858628])`; gr's helper is `lerp(min, max-1, rand()/32767)`
(cat14 finding 1). Fields: +0xC m_gang (CPedGroup*), +0x10 m_wanderTimer (start +0x10, interval +0x14, started +0x18, stopped +0x19), +0x1C m_scanTimer.

## 0x65DFF0 CTaskComplexGangLeader::CreateNextSubTask  (gr_deviations = 2)
- TaskComplexGangLeader.cpp:184 `m_wanderTimer.Start(GetRandomNumberInRange(0, 15000))`: Original 0x65E137-0x65E173: `k = _ftol(((rand() & 0xFFFF) * 3.0517578e-05f)
  * -15000.0f [0x86F934])`; `interval (+0x14) = 15000 - k` (i.e. 15000 + trunc(r*15000/32768) = 15000..29999, NOT 0..14999); `start (+0x10) = now`;
  `started (+0x18) = 1`. Verdict: CONFIRMED (different range AND formula: the original timer is 15..30 s). Class: keep-original.
- TaskComplexGangLeader.cpp:185 `new CTaskComplexWanderGang{PEDMOVE_WALK, (uint8)GetRandomNumberInRange(0, 8), 5000, true, 0.05f}`: Original 0x65E16B-0x65E1BF:
  `p = CTask::operator new(0x38)` FIRST; only if p != 0: `dir = _ftol((rand() & 0xFFFF) * 3.0517578e-05f * 8.0f [0x859000])`;
  `CTaskComplexWanderGang(p, 4, dir, 0x1388, 1, 0.05f)` 0x66F5C0. Verdict: CONFIRMED (formula; no rand when the pool is full). Class: keep-original.
- Also in this function (no marker): the "turn to face a member" branch draws an inline span roll `_ftol(r * 2^-15 * span)` with the span read from a local
  (0x65E044-0x65E062) before GetMember 0x5F69B0; and the stand-still branch uses a percent roll `<= 95` with a member count >= 3 (0x65E0C3-0x65E0EE).
  Not checked against gr line by line (UNCLEAR: 0x65E05C operand source).

## 0x65E200 CTaskComplexGangLeader::ScanForStuff  (gr_deviations = 5)
- cpp:387 `rndChance = GetRandomNumberInRange(0, 100)`: Original 0x65E260-0x65E27E inline percent roll (formula above). `< 5` -> lean branch (0x65E283), `== 20`
  (0x14) -> recruit branch (0x65E404). Verdict: CONFIRMED (formula). Class: keep-original.
- cpp:416 `new CEventLeanOnVehicle{&veh, GetRandomNumberInRange(10000, 25000)}`: Original `GetRandomNumberInRange(0x2710, 0x61A8)` 0x65E36A-0x65E374 BEFORE
  `CEvent::operator new(0x14)` 0x65E37D; `CEventLeanOnVehicle(p, veh, t)` 0x65DAF0 (null p passes 0 to the group event). Verdict: CONFIRMED. Class: keep-original.
- cpp:423 `#ifdef FIX_BUGS break; #endif`: Original: no break - after AddEvent 0x5F7470 and `m_scanTimer = {now, 60000, started}` (0x65E3CC-0x65E3E0) the loop continues
  over all 16 vehicle slots (0x65E3ED-0x65E3F9), so several lean events can be added in one scan. Verdict: CONFIRMED (original = no break). Class: keep-original.
- cpp:447 `#ifdef FIX_BUGS scannedPed.GetTaskManager().Find<CTaskComplexWander>() #else ped->...`: Original 0x65E49D-0x65E4AF:
  `FindActiveTaskByType(&leaderPed->m_pIntelligence->m_TaskMgr, 0x390)` on the LEADER (`[esp+0x7C]` = ped argument), then `GetWanderType()` (vtbl+0x2C) == 4 and
  `CTaskComplexWanderGang::CanJoinGang` 0x669FB0 == 0 -> skip this candidate. Verdict: CONFIRMED (original = #else branch). Class: keep-original.
- cpp:474 `partnerType = GetRandomNumberInRange(0, 7)`: Original 0x65E63A-0x65E63E `GetRandomNumberInRange(0, 7)` 0x407180 after the distance test
  (`fcomp 16.0f [0x8599D0]; test ah,1; jne skip` at 0x65E625-0x65E634); CEventCreatePartnerTask 0x5F6190 (type, closest, 1, 0.5f) added to the scanned ped
  then (type, scanned, 0, 0.5f) to the member; then `m_scanTimer = {now, 10000, started}` and return (0x65E6C2-0x65E6DA). Verdict: CONFIRMED (formula). Class: keep-original.

## 0x662370 CTaskComplexGangLeader::ControlSubTask  (gr_deviations = 3, plus 3 classified rows)
- cpp:214 notsa dyn_cast to CTaskComplexWander (0x66241F): Original checks `m_pSubTask->GetTaskType() == 0x390` exactly (GetTaskType called twice, first result
  discarded, 0x66240F-0x66241F). REFUTED (same as a type-based cast). harmless.
- cpp:221 (OG-bug comment; gr returns nullptr): Original 0x662425-0x6624C1: if the wander timer (+0x10) is out of time and
  `GetDistSqOfClosestPathNodeToPed(ped)` 0x669FF0 `< 2.0f [0x858CA0]` (STRICT: `test ah,5; jp`; gr uses `<=`): `SetDefaultTaskAllocatorType(m_gang->intelligence, 5)`
  0x5FBB70 then allocates `new CTaskSimpleStandStill` (0x20, 0x662486) and returns it. Verdict: CONFIRMED (gr returns nullptr, and `<=` vs `<`). Class: keep-original
  for the compare; the returned task follows the original (UNCLEAR whether the original really reaches a pure-virtual call afterwards; gr claims it aborts).
- cpp:257 LookAt `GetRandomNumberInRange(3000, 5000)` with `GetMembership().GetRandom()`: Original 0x662586-0x6625DC: only if not already looking (0x662579) and an
  inline percent roll `> 95` (0x6625A9); `t = (float)GetRandomNumberInRange(0xBB8, 0x1388)` then `m = GetMember(GetRandomNumberInRange(0, 8))` 0x5F69B0 (a random
  SLOT, may be empty). Verdict: CONFIRMED (helper formula and member selection; gr comment admits the change). Class: keep-original.
- cpp:284 `rnd = GetRandomNumberInRange(0, 500)`: Original 0x66291D-0x662933: `rnd = GetRandomNumberInRange(0, 0x1F4)`; `51 <= rnd <= 55` -> `new CTaskSimpleRunAnim(
  0x34, 0x117 + _ftol((rand() & 0xFFFF) * 3.0517578e-05f * 8.0f), 4.0f, 0)` 0x61A8B0 (rand only if the allocation succeeded, 0x662937-0x662985); else `cmp 100`
  (0x66298C). Verdict: CONFIRMED (formula). Class: keep-original.
- cpp:314 notsa any_of over anims: not re-checked (classified harmless).
- cpp:350 `if (GetRandomNumberInRange(0, 500) != 200)` + `if (CGeneral::DoCoinFlip())`: Original 0x66277B-0x66278E `GetRandomNumberInRange(0, 0x1F4) != 200` ->
  0x6628A7: `if (GetRandomNumberInRange(0, 100) != 50) return m_pSubTask` (0x6628A7-0x6628B6), i.e. a 1% chance, then FindTaskByType(0x133 HOLD_ENTITY) ...
  Verdict: CONFIRMED (helper formula) and, found in passing, CONFIRMED: gr's DoCoinFlip (50%) replaces the original's `== 50` 1% roll. Class: keep-original.
