# CBulletTraces (RENDER) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/BulletTraces.cpp` (pin 20d8d2a). aTraces 0xC7C748, 16 x 0x2C (+0 m_vecStart, +0xC m_vecEnd,
+0x18 m_bExists, +0x1C m_nCreationTime, +0x20 m_nLifeTime, +0x24 m_fRadius, +0x28 m_nTransparency).

## 0x723750 CBulletTraces::AddTrace (from, to, radius, disappearTime, alpha)  (gr_deviations = 2)
Original: `__cdecl void (const CVector* from [esp+4], const CVector* to [esp+8], float radius [+0xC], uint32 disappearTime [+0x10],
uint8 alpha [+0x14])`, plain `ret` from several exits (0x723B8C/0x723BA1/0x723BE9/0x723BFE/0x723C06).
Rows BulletTraces.cpp:147 and :148 (rng-helper: `GetRandomNumberInRange(0.0f, traceLengthOriginal)` / `(2.0f, 5.0f)`).
- Original bytes: these lines belong to the OTHER overload `AddTrace(posMuzzle, posBulletHit, weaponType, fromEntity)` at 0x726AF0
  (gr BulletTraces.cpp:121); 0x723750 contains no rand/CGeneral call (callees: only `_ftol` 0x821B40 and
  CAudioEngine::ReportFrontendAudioEvent 0x506EA0).
- Verdict: REFUTED for this function (rows misattributed by name; they must be checked under 0x726AF0). Class: harmless here.
Found in passing (CONFIRMED, keep-original):
1. Lifetime: the original first COUNTS the existing traces (`m_bExists` of all 16, unrolled x8, 0x723753-0x7237B4) and uses the count,
   not the index of the slot it fills: `count >= 10` -> `life = _ftol((float)disappearTime * 0.25f)` ([0x858C84]; unsigned->float via
   fild + 4294967296.0f [0x858C54] when negative, 0x7237BE-0x7237D6); `count >= 5` -> `_ftol((float)disappearTime * 0.5f)` (0x7237DF-0x7237FC);
   else `disappearTime`. gr uses `GetTraceIndex(pTrace)` (index of the first free slot), which differs when slots are fragmented.
   Then the first free slot i (0x723809-0x72388F; none -> skip the store) gets: start = *from, end = *to, creationTime = now,
   transparency = alpha, exists = 1, radius, lifeTime (store order 0x72389C-0x7238F0).
2. Pass-by sound (0x7238F7-0x723C06; gr PlayFrontEndSoundForTrace): the volume is `20.0f * log10(v)` (`fldlg2; fyl2x; fmul [0x858BA4]=20.0`,
   0x723B56-0x723B5C / 0x723BB3-0x723BB9) with `v = 1.0f - dist * 0.5f`, or -100.0f when `v == 0` (0x723B4C); gr passes `std::log10(v)`
   without the factor 20. The sign test is `fromY * toY` compared with 0.0 (0x723A79-0x723A8A: returns unless the product is < 0), gr uses
   `signbit` equality (differs for +-0). `dist >= 2.0f` (or NaN) returns (0x723B0F-0x723B1A).
Open questions: which camera-space value is compared at 0x723B20 (selects events 0x1B/0x1C vs 0x1D/0x1E) and at 0x723B66/0x723BC3
(f64 0.0 [0x859EF8], selects REAR vs FRONT) was not traced through the x87 stack here; gr maps `x >= 0` to RIGHT (0x1D/0x1E) while the
original sends `!(value < 0)` to 0x1B/0x1C (LEFT_*). Confirm the operand before porting this part.
