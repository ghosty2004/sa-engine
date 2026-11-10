# CGangWars (POP) - gta-reversed deviation notes (group P4)

Source: `re/gta-reversed/source/game_sa/GangWars.cpp` (pin 20d8d2a). Globals (gr names, GangWars.h): Gang2 0x96AB50 (i32), WarFerocity
0x96AB58, Provocation 0x96AB5C (f32), TimeStarted 0x96AB60, State 0x96AB64, Difficulty 0x96AB80 (f32), Gang1 0x96AB84, pZoneToFightOver
0x96AB88, pZoneInfoToFightOver 0x96AB8C, bTrainingMission 0x96AB91, TerritoryUnderControlPercentage 0x96AB9C (f32), bIsPlayerOnAMission
0x96ABA0, GangRatingStrength[3] 0x96AB2C, GangRatings[3] 0x96AB38. CZoneInfo (0x11 bytes): GangStrength[10] +0x00 (u8; 0 BALLAS, 1 GROVE,
2 VAGOS ...), ZoneColor +0x0B..+0x0E (r,g,b,a), +0x0F bits 5-6 RadarMode. Ped flags dword +0x474: bit 0 bDonePositionOutOfCollision,
bit 12 (0x1000) bPartOfAttackWave, bit 13 (0x2000) bClearRadarBlipOnDeath. Pool loops: `for (i = size; i-- > 0;)` (descending), slot
skipped when `flags[i] & 0x80` (free); ped stride 0x7C4, vehicle stride 0xA18. Ped handle = `(index << 8) + flags[index]` (CPools::GetPedRef
inlined: `(ptr - base) / 0x7C4` via magic 0x41EE7CA7). `_rand` 0x821B1E, `_ftol` 0x821B40, `CGeneral::GetRandomNumberInRange(int)` 0x407180.
Constant 0x858C7C = 3.0518509e-05 (bit pattern 0x38000100, approx 1/32767).

## 0x443DB0 CGangWars::CalculateTimeTillNextAttack  (gr_deviations = 1)
Original: `__cdecl`, no args. `t = (float)rand() * 3.0518509e-05 [0x858C7C]; t *= 0.9f [0x858C20]; t *= 1080000.0f [0x8598B8];
t += 648000.0f [0x8598B4]` (x87, 0x443DBA-0x443DD0), then tail `jmp _ftol` (0x443DD9): the result is an INTEGER (truncated) in EAX
(EDX = high dword), not a float. Range 648000 .. 648000 + 972000 = 1620000.
Row GangWars.cpp:139 `return CGeneral::GetRandomNumberInRange(648'000.0f, 1'620'000.0f)` (rng-helper).
- Original bytes: inline `rand * k * 0.9 * 1080000 + 648000` with the 1/32767 constant (not the 0x41BD90 helper, not 1/32768), then ftol.
- Verdict: CONFIRMED. gr uses its float lerp helper (different formula and rounding) and returns `float`; the original returns the
  truncated int. Class: keep-original.

## 0x443DE0 CGangWars::UpdateTerritoryUnderControlPercentage  (gr_deviations = 2)
Original (`__cdecl void ()`): counters grove = esi, ballas = [esp+0x14], vagos = [esp+0x10], all 0 (0x443DED-0x443DFE).
Loop `n = CTheZones::TotalNumberOfNavigationZones` (u16 0xBA3794) times over NavigationZoneArray (`+0x1C` m_ZoneInfoIndex u16 read at
0xBA37B4 + 0x20*k, 0x443E10): skip when index == 0 (0x443E16). With `zi = 0xBA1DF0 + 0x11*index` (ZoneInfoArray): `B = zi[0]`, `G = zi[1]`,
`V = zi[2]` (u8). `s = B + V`; if `s == 0 && G == 0` skip (0x443E39-0x443E3D); else if `s <= G` (signed, 0x443E41) `grove++`; else if
`B > V` (unsigned, 0x443E45) `ballas++` else `vagos++` (ties go to vagos, 0x443E4D).
Then `CStats::SetStatValue(0xEC, (float)grove)` (0x55A070); record update inlined: `if (GetStatValue(0xED) < (float)grove)` (0x443E8F,
`test ah,5; jp`: NaN/>= take the else) value = (float)grove else value = GetStatValue(0xED) (second call); `SetStatValue(0xED, value)`
(0x443EC2, always called). No CheckForStatsMessage.
`total = vagos + ballas + grove`; if 0: TerritoryUnderControlPercentage = 0 (int store, 0x443EDA) and return. Else
`pct = (float)grove (f32 temp) / (float)total` (`fild total; fdivr`, 0x443EE4-0x443EF1) stored f32 to 0x96AB9C; then a bubble sort of
three (strength, gang id) slots, initial order slot0 = (grove, 1), slot1 = (ballas, 0), slot2 = (vagos, 2) (also stored once at
0x443EF5-0x443F1C): repeat { swapped = 0; if s0 < s1 (signed) swap slots 0/1, swapped = 1; if s1 < s2 { swap 1/2; restart } ; } until
!swapped (0x443F28-0x443F50). Store GangRatings[rank] = gang id (0x96AB38/3C/40), GangRatingStrength[rank] = count (0x96AB2C/30/34).
Rank 0 = largest; ties keep the initial order grove, ballas, vagos (stable, strict `<`).
Row GangWars.cpp:970 ("todo: make more original"):
- Original bytes: the counting rules above differ from gr: (a) gr counts grove only when `G > B + V` (strict); the original counts grove when
  `G >= B + V` (0x443E41 `jle`), except when all three are 0. (b) gr counts ballas only when `B > V` and vagos only when `V > B` (ties
  counted nowhere); the original counts every non-grove zone, ties as vagos (0x443E45). (c) gr iterates the whole NavigationZoneArray;
  the original iterates TotalNumberOfNavigationZones entries (same result only if unused entries have index 0). (d) gr calls
  `CStats::SetNewRecordStat`, which also runs CheckForStatsMessage; the original inlines the compare and always calls SetStatValue(0xED).
- Verdict: CONFIRMED. Class: keep-original.
Row GangWars.cpp:1011 (NOTSA ranking):
- Original bytes: 0x443F28-0x443F78 store gang ids indexed by rank (`GangRatings[rank] = gang`). gr writes `GangRatings[e.gang] = i`
  (rank indexed by gang: the inverse mapping) and sorts with unstable `std::sort` from the initial order {BALLAS, GROVE, VAGOS} (original:
  stable, {GROVE, BALLAS, VAGOS}).
- Verdict: CONFIRMED (gr logic error: the GangRatings array has the inverse meaning). Class: keep-original.

## 0x4444B0 CGangWars::ClearTheStreets  (gr_deviations = 1)
Original (stub `jmp 0x1564200`, body in .HOODLUM): ped pool descending; skip free slots, `CPed::IsPlayer` (0x5DF8F0), and peds whose
`+0x598` m_nPedType is not 4 or 5 (0x1564246-0x156424E); `t = intelligence(+0x47C)->taskManager(+4).FindActiveTaskByType(0x390)`
(0x681740); if t: `t+0x0C = 7` (PEDMOVE_SPRINT, 0x1564267).
Row GangWars.cpp:194 (pool-iteration):
- Original bytes: descending loop 0x156421C-0x1564270. Each ped is independent and nothing random is consumed, so the order is not
  observable. gr's `Find<CTaskComplexWander>()` must resolve to FindActiveTaskByType(TASK_COMPLEX_WANDER 0x390) on the active task.
- Verdict: REFUTED (order not observable). Class: harmless.

## 0x444530 CGangWars::TellGangMembersTo  (gr_deviations = 2)
Original (stub `jmp 0x1568E00`, .HOODLUM body, SEH frame; arg [esp+4] read as a DWORD at 0x1568EAF): ped pool descending. EBX is set by
the SecuROM dispatcher (`call 0xCB73E0` with table 0x155FFB0, `or ebx, eax`, 0x1568E49-0x1568E4C) and then used as the
FindPlayerPed argument and the KillPedOnFoot second argument; gr uses -1 for both (consistent, but the value is not statically readable,
see open question). Skip free slots, IsPlayer, and peds with type < 7, > 16 or == 8 (0x1568E8E-0x1568EA9).
`switch (arg)`: 0 -> `new CTaskComplexKillPedOnFoot(FindPlayerPed(ebx), ebx, 0, 0, 0, 2)` (0x620E30) wrapped in
`CEventScriptCommand(3, task, 0)` (0x4B0A00) and `CEventGroup::Add(&ev, 0)` on intelligence+0x68; 1 -> see below; any other value -> nothing.
For 1: if `+0x46C & 0x100` (in vehicle): if `CVehicle::IsDriver(ped)` (0x6D1C40) task = `new CTaskComplexCarDriveWander(veh +0x58C, 0, 10.0f)`
(0x63CB10) else task = `new CTaskSimpleCarDrive(veh, 0, 0)` (0x63C340); post it and go to the NEXT ped (0x1568F58/0x1568FD1 jump to the
event destructor 0x15690CA). Only when NOT in a vehicle: task = `new CTaskComplexWanderGang(4, GetRandomNumberInRange_int(0, 8) [0x407180],
5000, 1, 0.5f [0x86FCB0])` (0x66F5C0), posted.
Row GangWars.cpp:680 (pool-iteration):
- Original bytes: descending 0x1568E51-0x15690DC. The only RNG consumer is the wander-gang path; the order of rand() calls follows the
  slot order, so ascending iteration changes which ped gets which random value.
- Verdict: CONFIRMED (order observable through rand). Class: keep-original (descending).
Row GangWars.cpp:707 (rng-helper):
- Original bytes: 0x1569006 `call 0x407180` (int helper, args 0, 8).
- Verdict: CONFIRMED. Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr posts a CTaskComplexWanderGang to EVERY ped when the war ends, including peds in
vehicles after their drive task; the original posts the drive task only (in-vehicle peds get no wander task).

## 0x4453D0 CGangWars::CreateDefendingGroup  (gr_deviations = 7)
Rows GangWars.cpp:240/241 (rng-helper, ped position):
- Original bytes (0x445573-0x44560F): `r1 = (float)rand() * 3.0518509e-05 [0x858C7C]`; `angle = (float)i * 6.2831855f [0x858CBC] / (int)count`
  (`fidiv`, stored f32 with `fst`, 0x445594); `x = sin(angle) * (r1 * 3.0f [0x858B3C] + 2.0f [0x858CA0]) + node.x` (stored f32);
  `r2` likewise; `y = cos(angle from the f32) * (r2 * 3.0f + 2.0f) + node.y`; `z = node.z + 2.0f`; then
  `z = CWorld::FindGroundZFor3DCoord(x, y, z, 0, 0) + 1.0f [0x858624]` (0x445604).
- Verdict: CONFIRMED, three differences: gr's helper formula; gr adds `+ 2.0f` OUTSIDE the product (`sin * rand(0,3) + 2`), the original
  multiplies by `(rand*3 + 2)`; gr drops the `+ 1.0f` after FindGroundZFor3DCoord. Class: keep-original.
Row GangWars.cpp:267 (NOTSA_UNREACHABLE in the weapon switch):
- Original bytes: `i % 4` signed (0x44575A-0x445767), `ja 0x4457C1` for > 3 (unreachable since i >= 0) skips both GiveDelayedWeapon and
  SetCurrentWeapon; table 0x445B20 = {0x445774, 0x445786, 0x445798, 0x4457AA} -> weapons 22, 28, 25, 29, each
  `GiveDelayedWeapon(w, 5000)` (0x5E89B0) then `SetCurrentWeapon(w)` (0x5E6280).
- Verdict: REFUTED (unreachable in both). Class: harmless.
Rows GangWars.cpp:307/308 (rng-helper, pickup position):
- Original bytes (0x445A4D-0x445AC5): `x = r * 3.0518509e-05 * 4.0f [0x858B90] + node.x - 2.0f [0x858CA0]` (order: product, + node.x, - 2),
  y likewise; `z = FindGroundZFor3DCoord(x, y, node.z + 1.0f, 0, 0) + 0.75f [0x858F34]`.
- Verdict: CONFIRMED (helper formula; gr's `rand + node - 2` order matches). Class: keep-original.
Row GangWars.cpp:314 (rng-helper, pickup model):
- Original bytes: 0x445AC9 `call rand; test al, 1`: odd -> MI_PICKUP_HEALTH (0x8CD5C8), even -> MI_PICKUP_BODYARMOUR (0x8CD5C0). No range
  helper at all; `GenerateNewOne(pos, model, 5, 0, 0, 0, 0)` (0x456F20).
- Verdict: CONFIRMED (gr uses GetRandomNumberInRange(0, 2) == 1, a different function of the same rand value). Class: keep-original.
Row GangWars.cpp:1032 (GetGangColor, NOTSA helper for the inlined code at 0x4457D5-0x445801):
- Original bytes: colour = `T1[g] << 24 | T2[g] << 16 | T3[g] << 8 | 0xFF` with T1 = 0x8D1344 {C8 46 FF 00 FF C8 F0 00 FF FF 00 00},
  T2 = 0x8D1350 {00 C8 C8 00 DC C8 8C C8 FF FF 00 00}, T3 = 0x8D135C {C8 00 00 C8 BE C8 F0 FF FF FF 00 00}. gr's arrays (R, then the one
  named G = T2, the one named B = T3) give the same value (its comment's table/name pairing is confusing but the result matches).
- Verdict: REFUTED. Class: harmless.
Found in passing: the per-ped flag write is `+0x474 |= 0x3000` (0x4457C9), blip `SetEntityBlip(2, handle, colour, 2, 0x859908)`,
`ChangeBlipScale(blip, 2)`, `ChangeBlipColour(blip, colour)`; matches gr.

## 0x445B30 CGangWars::AttackWaveOvercome  (gr_deviations = 1)
Original: stub `sub esp,0x14; push ebx; nop; jmp 0x401DCB` (EBX = ped pool via an obfuscated load, 0x401DCC-0x401DDC). Descending loop;
skip free and `!(+0x474 & 0x1000)`. If `+0x530` m_nPedState is 0x37 or 0x36: `+0x474 &= ~0x1000` and `SetCharCreatedBy(1)` (0x445BED-0x445BFA).
Else `living++`; `if (sqrt(dx*dx + dy*dy) < 45.0f [0x858CB4])` near++ (d = ped pos - FindPlayerCoors(-1); `test ah,5; jp`, NaN not near).
Return `(living <= 1 && near == 0)` as EAX 1/0 (0x445C07-0x445C24).
Row GangWars.cpp:117 (pool-iteration):
- Original bytes: descending 0x445B60-0x445C01; result is order independent.
- Verdict: REFUTED for the order. Class: harmless.
Found in passing (CONFIRMED, keep-original): for dying peds gr also clears `bDonePositionOutOfCollision` (bit 0 of +0x474); the
original clears only bit 12 (`and eax, 0xFFFFEFFF`, 0x445BED).

## 0x445C30 CGangWars::ReleasePedsInAttackWave  (gr_deviations = 1)
Original: stub `jmp 0x15609C0` (.HOODLUM, SEH). bl = arg2 `restoreAcquaintance` (0x15609F5), arg1 `isEndOfWar` read at 0x1560B4F. A SecuROM
dispatcher (`call 0xCB73E0` with table 0x155FF14) returns a value placed in EBP (0x1560A22) that is used as the AND mask
clearing bClearRadarBlipOnDeath (0x1560AB8, 0x1560BA4); gr uses `~0x2000` (see open question). Descending loop: if slot free, `esi = 0`.
If occupied and `+0x474 & 0x1000`: clear 0x1000, `SetCharCreatedBy(1)`, count++, `CRadar::ClearBlipForEntity(2, handle)` (0x587C60,
handle = `(idx << 8) + flags[idx]`), `+0x474 &= ebp`; if restore: `CTaskComplexWander::GetWanderTaskByPedType(ped)` (0x673D00) posted via
CEventScriptCommand(3, task, 0) / CEventGroup::Add; acquaintance (5 dwords at +0x4E0) = `*CPedType::GetPedTypeAcquaintances(+0x598)`
(0x6089B0). Then (for any occupied slot) if isEndOfWar and `+0x474 & 0x2000`: ClearBlipForEntity(2, handle) and `+0x474 &= ebp`.
Returns the count in EAX.
Row GangWars.cpp:526 (pool-iteration):
- Original bytes: descending 0x1560A2A-0x1560BAE. Order is not observable.
- Verdict: REFUTED for the order. Class: harmless.
Found in passing (CONFIRMED, keep-original): gr passes the slot index `i` to ClearBlipForEntity; the original passes the pool HANDLE
`(i << 8) + flags[i]` (0x1560A81-0x1560AAD, 0x1560B66-0x1560B96). gr's blip lookup uses the wrong id.
Open question: EBP mask value at 0x1560A22 (produced by the SecuROM stub 0xCB73E0, not statically readable; behaviour consistent with ~0x2000).

## 0x445E20 CGangWars::ReleaseCarsInAttackWave  (gr_deviations = 1)
Original: vehicle pool descending (stride 0xA18); if `+0x42D & 8`: clear bit 3, `CVehicle::SetVehicleCreatedBy(1)` (0x6D5D70).
Row GangWars.cpp:513 (pool-iteration):
- Original bytes: descending 0x445E38-0x445E6B; order-independent.
- Verdict: REFUTED. Class: harmless.

## 0x446050 CGangWars::StartOffensiveGangWar  (gr_deviations = 1)
Original (`__cdecl void ()`): CoorsOfPlayerAtStartOfWar (0x96ABBC) = FindPlayerCoors(-1); `pZoneInfoToFightOver = CTheZones::GetZoneInfo(&coors,
&pZoneToFightOver)` (0x572400); Difficulty = TerritoryUnderControlPercentage (raw copy); if `!CanPlayerStartAGangWarHere(zi)` (0x443F80):
Provocation = 0, return (0x4463E2).
Row GangWars.cpp:613 (NOTSA Gang1/Gang2/density sum):
- Original bytes (0x4460B0-0x446256): a running top-two scan over GangStrength indices 0, 2, 3, ..., 9 (index 1 = GROVE is never read):
  max1 = max2 = 0. Index 0: if s0 > 0 { Gang2 = Gang1 (old global value); Gang1 = 0; max1 = s0 } (else Gang1/Gang2 keep their old
  global values). Each later i: if `s_i > max1` { Gang2 = Gang1; Gang1 = i; max2 = max1; max1 = s_i } else if `s_i > max2`
  { max2 = s_i; Gang2 = i } (strict, signed). sum = s0 + s2 + ... + s9 (grove excluded). Provocation = 0 (0x446262).
  Continue only if `sum > 0` and Gang1 is 0 or 2 and State2 (0x96AB4C) == 0. Then GW_PROV (0x859920) AddMessageQ(text, 4500, 1, 1)
  and AddToPreviousBriefArray(text, -1 x6, 0); TimeStarted = time; State = 1; ClearTheStreets; WarFerocity = min(sum / 15 (signed), 2).
  `if (GetStatValue(0xB5) < 0.0f [0x858B50])`: if `pct <= 0.4f [0x858EE8]` WarFerocity = 0 (0x44633D `test ah,0x41; jp`), else if
  `!(pct < 0.5f [0x858B8C])` WarFerocity = 5 (0x446358; NaN keeps). `if (bTrainingMission [0x96AB91]) WarFerocity = 0`.
  `if (max1 > max2 / 3 (signed))` Gang2 = Gang1 (0x446376-0x44638E). TellGangMembersTo(0); zone +0x0F = (b & 0xDF) | 0x40; colour
  (255, 0, 0, 160); pDriveByCar (0x96AB28) = 0.
- Verdict: CONFIRMED, several differences: gr includes GROVE in the max and in the sum; gr recomputes Gang2 as the index of the second
  largest (with zero ties picking index 0/1) while the original keeps stale globals when strengths are 0 and only updates Gang2 inside the
  scan; gr's ferocity override is `pct > 0.4 ? (pct > 0.5 ? 5 : 0) : unchanged`, the original is `pct <= 0.4 -> 0; pct >= 0.5 -> 5;
  otherwise unchanged`; gr tests bIsPlayerOnAMission (0x96ABA0), the original bTrainingMission (0x96AB91). Class: keep-original.
