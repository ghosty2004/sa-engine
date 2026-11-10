# CCarCtrl (POP) - gta-reversed deviation notes

Source: `re/gta-reversed/source/game_sa/CarCtrl.cpp`. RNG background: docs/re/cat14-gr-deviations.md finding 1
(gr integer `GetRandomNumberInRange(min,max)` = `(T)lerp(min, max-1, rand()/32767.0f)`; gr float version =
`lerp(min,max,t)` = `max*t + min*(1-t)` with `t = rand() * (1/32767.0f)`). `_rand` = 0x821B1E (CRT, returns
0..32767), `_ftol` = 0x821B40 (CRT, truncation toward zero, pops ST0).

## 0x4212E0 CCarCtrl::Init  (gr_deviations = 1)
Original: `__cdecl void ()`. Stores 1.0f (imm 0x3F800000) to CarDensityMultiplier 0x8A5B20 (0x4212E5); 0 to NumRandomCars
0x969094 .. LastTimeFireTruckCreated 0x9690B4 (9 dwords, 0x4212EF-0x42131F); apCarsToKeep[0..1] 0x969084/0x969088 = 0
(0x421334, 0x421353); bAllowEmergencyServicesToBeCreated 0x8A5B28 = 1 (0x421339); bCarsGeneratedAroundCamera 0x9690C1 = 0
(0x421346); CountDownToCarsAtStart 0x9690C0 = 2 (0x42134C); then `CLoadedCarGroup::Clear` 0x611B90 (thiscall) on every
gang group from 0xC0EA28 in steps of 0x2E while `esi < 0xC0EC22` (signed compare, 0x42137A: 11 groups, the last at
0xC0EC00), then on
m_AppropriateLoadedCars 0xC0E9F8, m_InAppropriateLoadedCars 0xC0E9C8, and m_LoadedBoats 0xC0E998 (tail jmp 0x4213A0).
gr order of the stores differs but no store aliases another, so order is harmless.
Found in passing (CONFIRMED, keep-original): gr declares `m_LoadedGangCars` as `std::array<CLoadedCarGroup, 10>` (Population.h:43)
and clears 10 groups; the original loop bound (0xC0EC22 = &bZoneChangeHasHappened) clears 11 groups of 0x2E bytes, so the array
really has 11 entries (0xC0EA28 + 11*0x2E = 0xC0EC22). The port must clear the group at 0xC0EC00 too.

Row CarCtrl.cpp:163 `TimeNextMadDriverChaseCreated = CGeneral::GetRandomNumberInRange(600.0f, 1200.0f);`
- Original bytes: `r = _rand()` (0x421325, NOT masked with 0xFFFF here); `fild r` (0x42132E); `* 0x858C7C = 3.0518509447574615e-05`
  (f32, = 1/32767, 0x421340); `* 0x858EDC = 600.0` (0x421358); `+ 0x858EDC = 600.0` (0x421363); `fstp` f32 to
  TimeNextMadDriverChaseCreated 0x9690BC (0x421369). I.e. inline `(r * (1/32767)) * 600 + 600`, no call to 0x41BD90.
- gr: `lerp(600, 1200, t)` = `1200*t + 600*(1-t)`. Same value in exact arithmetic, different rounding (24-bit x87 precision
  in the original).
- Verdict: CONFIRMED (rounding-level). Class: keep-original - port `((r as f32 * 1/32767f) * 600f) + 600f` with the x87 order above.

## 0x421900 CCarCtrl::ChooseCarModelToLoad  (gr_deviations = 1)
Original: `__cdecl int32 (int32 groupId [esp+4])`, returns EAX, plain `ret`.

Row CarCtrl.cpp:209 `const auto model = CPopulation::m_CarGroups[groupID][CGeneral::GetRandomNumberInRange(numCarsInGroup)];`
- Original bytes: `n = (int32)(int16)m_nNumCarsInGroup[groupId]` (0xC0EC78 + 2*g, 0x421911, re-read every iteration).
  Loop `for (tries = 0; tries < 16; tries++)` (ebp, 0x421959-0x42195D): `r = _rand() & 0xFFFF` (0x421922);
  `idx = _ftol( n * (r * 0x858B14) )` with 0x858B14 = 3.0517578125e-05 (f32, = 1/32768; x87: `fild r; fmul [0x858B14];
  fild n; fmul st(1)`, 0x42192B-0x421939; `_ftol` 0x42193B, the leftover `r*k` popped at 0x421940);
  `model = (uint16)m_CarGroups[g*23 + idx]` (0xC0ED38, row stride 0x17 halfwords, 0x42190E/0x421944);
  if `CStreaming::ms_aInfoForModel[model].m_nLoadState` (byte at 0x8E4CD0 + 20*model) `!= 1` (LOADSTATE_LOADED) return
  `model` (0x42194F-0x421957). After 16 tries return -1 (0x42195F).
- gr: index `(int)lerp(0, n-1, rand()/32767.0f)` - different index for the same rand() (top index only at rand()==32767).
- Verdict: CONFIRMED. Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr wraps the loop in `if (numCarsInGroup > 0)` and returns -1 otherwise. The
original has no such check: with n == 0 every try uses idx 0 and returns `m_CarGroups[g][0]` if that model is not loaded
(and with n < 0 the truncation gives idx <= 0, possibly a negative index into the previous group row). The original also
consumes 16 rand() calls in the all-loaded case; gr consumes none when n <= 0.

## 0x421980 CCarCtrl::ChoosePoliceCarModel  (gr_deviations = 1)
Original: `__cdecl int32 (uint32 ignoreCopBike [esp+4])`, returns EAX. Model-loaded tests read `ms_aInfoForModel[m].m_nLoadState`
(0x8E4CD0 + 20*m) `== 1` directly: 0x8E6E2C = ENFORCER 427, 0x8E6314 = SWAT 285, 0x8E7318 = FBIRANCH 490, 0x8E6328 = FBI 286,
0x8E6E90 = RHINO 432, 0x8E6EA4 = BARRACKS 433, 0x8E633C = ARMY 287. `FindPlayerWanted(-1)` 0x56E230 is called anew before each
of AreSwatRequired 0x561F40 / AreFbiRequired 0x561F60 / AreArmyRequired 0x561F80.
1. If Swat required && ENFORCER loaded && SWAT loaded (0x42198D-0x4219A6): `if (GetRandomNumberInRange_int32(0, 3) (0x407180) == 2)
   return 427` (0x4219B9) else tail-jump to `CStreaming::GetDefaultCopCarModel` 0x407C50 with the caller's own argument still at
   [esp+4] (0x421A33). Any failing condition of step 1 falls to step 2.
2. If Fbi required && FBIRANCH loaded && FBI loaded: return 490 (0x4219E5).
3. If Army required && RHINO && BARRACKS && ARMY loaded: `return 432 + (_rand() < 0x3FFF)` (signed compare, 0x421A20-0x421A29).
4. Tail-jump GetDefaultCopCarModel(ignoreCopBike).
The control flow matches gr (lines 225-246).

Row CarCtrl.cpp:233 `if (CGeneral::GetRandomNumberInRange(0, 3) == 2)`
- Original bytes: `call 0x407180` with (0, 3) (0x4219A8-0x4219AC) = `0 + _ftol((rand()&0xFFFF) * (1/32768) * 3)`: 2 for
  rand() >= 21846, i.e. ENFORCER about 1/3 of the time.
- gr: `(int)lerp(0, 2, rand()/32767.0f)` is 2 only when rand() == 32767: gr practically never picks ENFORCER here.
- Verdict: CONFIRMED (behaviourally significant). Class: keep-original.

## 0x426220 CCarCtrl::SlowCarDownForObject  (gr_deviations = 1)
Original: `__cdecl void (CEntity* entity [esp+4], CVehicle* vehicle [+8], float* speedMult [+0xC], float maxMult [+0x10])`,
plain `ret`. Uses the vehicle's `m_matrix` (+0x14) directly for the forward/right vectors without allocating it (0x426229;
null would crash); positions are `m_matrix ? &m_matrix->pos : &placement.pos` for both entities (0x426229-0x426246).
`d = entityPos - vehiclePos` (x, y stored f32 at 0x42624D/0x426257, z kept by `fst` 0x426261).
`h = (d.z*fwd.z + d.y*fwd.y) + d.x*fwd.x` (fwd = matrix+0x10..0x18; stored f32 into the vehicle arg slot, 0x42627A).
Continue only if `h > 0.0f` (0x858B50, `test ah,0x41`, 0x426286) and `h < 20.0f` (0x858BA4, `test ah,5; jp`, 0x42629B; NaN fails).
`s = fabs((d.z*right.z + d.y*right.y) + d.x*right.x)` (right = matrix+0..8, 0x4262A4-0x4262D5); continue only if
`entityColModel->boundSphere.radius (+0x24) + vehicleColModel->boundBox.max.x (+0x0C) > s` (fcompp, 0x4262E0-0x4262E7; colmodels
via `ms_modelInfoPtrs[modelIndex (+0x22, i16)]->+0x14`). Then if `h < 7.0f` (0x858F48, 0x4262ED): `*speedMult = 0`
(0x4262FF). Else `t = (1.0f (0x858624) - (h - 7.0f) * 0.07692307978868484f (0x858FA4)) * maxMult` (0x42630A-0x426324, x87:
`fsub 7; fmul k; fsubr 1.0; fmul arg`), and `*speedMult = (*speedMult < t) ? *speedMult : t` (0x426328-0x42633D; equal or NaN
stores t).

Row CarCtrl.cpp:880 `*arg3 = std::min(*arg3, (1.0f - (entityHeading - 7.0f) / 13.0f)) * arg4; // Original code multiplies by 0.07692308`
- Original bytes: `fmul dword [0x858FA4]` = 0.07692307978868484 (f32) at 0x426318, not a division by 13.
- Verdict: CONFIRMED (rounding-level: `x * f32(1/13)` vs `x / 13.0f`). Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr multiplies `arg4` AFTER the `min` (`min(cur, f) * arg4`); the original multiplies
inside (`min(cur, f * arg4)`, 0x426324 multiplies before the compare at 0x42632E). Different result whenever `maxMult != 1` -
a gr logic error. gr also uses `vehicle->GetMatrix()` (which allocates a matrix when missing) where the original reads `m_matrix`
unguarded.
