# CPopulation (POP) - gta-reversed deviation notes (group P4)

Source: `re/gta-reversed/source/game_sa/Population.cpp` (pin 20d8d2a). Counters: ms_nNumGang[10] 0xC0EC3C, ms_nNumDealers 0xC0EC38,
ms_nNumEmergency 0xC0EC64, ms_nNumCop 0xC0EC68, ms_nNumCivFemale 0xC0EC6C, ms_nNumCivMale 0xC0EC70, ms_nTotalPeds 0xC0EC28,
ms_nTotalGangPeds 0xC0EC2C, ms_nTotalCivPeds 0xC0EC30, CurrentWorldZone 0xC0FCBC. Pool loops: `for (i = size; i-- > 0;)` (descending),
skipping slots whose flag byte has bit 7 set; strides ped 0x7C4, object 0x19C, dummy 0x38. "delete" = `CWorld::Remove(p)` (0x563280) then
`p->vtable[0](1)` (scalar deleting destructor). `CGeneral::GetRandomNumberInRange(int)` 0x407180; float version 0x41BD90.

## 0x5B6D40 CPopulation::FindPedRaceFromName  (gr_deviations = 1)
Original (`__cdecl int (const char* name)`): for i = 0, 1: `c = name[i]`, if 'a' <= c <= 'z' then c -= 0x20; `k = c - 'B'`; if k > 0x15
(unsigned) continue; byte table 0x5B6DA4 / jump table 0x5B6D90: 'B' -> 1, 'H' -> 4, 'I' and 'O' -> 3, 'W' -> 2, others continue. Return 0.
Row Population.cpp:125 (`case 0: return RACE_DEFAULT; // Not handled properly originally`):
- Original bytes: a NUL at name[0] gives k = -0x42 (unsigned > 0x15), so the loop continues and reads name[1] (past the terminator).
- Verdict: CONFIRMED (gr stops at the NUL; the original also classifies by the byte after it). Class: keep-original.

## 0x611560 CPopulation::AddExistingPedInCar  (gr_deviations = 1)
Original: `mov eax, [esp+4]; ret` - `__cdecl CPed* (CPed* ped, CVehicle*)` returning its first argument.
Row Population.cpp:487 (NOTSA_UNREACHABLE): Verdict: CONFIRMED (gr aborts; the original returns `ped`). Class: keep-original.

## 0x611570 CPopulation::UpdatePedCount  (gr_deviations = 1)
Original (`__cdecl void (CPed* ped, bool removed)`; flags via stub 0x407005 `mov eax, [ecx+0x474]`): if removed: require `+0x474 & 4`
(bHasBeenAddedToPopulation), clear it; else require it clear, set it. Then by `+0x598` (type - 4, `ja` > 0x12 does nothing; byte tables
0x611650 (add) / 0x611680 (remove), jump tables 0x611634 / 0x611664): 4, 20 -> CivMale; 5, 22 -> CivFemale; 6 -> Cop; 7..16 ->
`[0xC0EC20 + 4*type]` (= ms_nNumGang[type - 7], since ms_nNumGang = 0xC0EC3C = 0xC0EC20 + 4*7); 17 -> Dealers; 18, 19 -> Emergency;
21 -> nothing. Increment when adding, decrement when removing.
Row Population.cpp:528 (FIX_BUGS `ms_nNumGang[type - PED_TYPE_GANG1]`, `#else ms_nNumGang[type]`):
- Original bytes: 0x6115C8 / 0x611615 address `0xC0EC20 + 4*type`, i.e. the FIX_BUGS form. gr's `#else` branch (claimed original) is a
  mis-transcription.
- Verdict: REFUTED (gr's default FIX_BUGS build matches the bytes). Class: keep-original.

## 0x611900 CPopulation::FindSpecificDriverModelForCar_ToUse  (gr_deviations = 1)
Original (`__cdecl int (int carModel)`): `k = carModel - 409`, `ja` > 0x48 -> -1; byte table 0x611984, jump table 0x611964: 409 -> 255;
420, 438 -> tail `jmp CStreaming::GetDefaultCabDriverModel` 0x407D50; 423 -> 264; 428 -> 71; 448 -> 155; 481 -> 23; 463 ->
`switch (GetRandomNumberInRange(0, 3))` (0x407180): 0 -> 247, 1 -> 248, else -1.
Row Population.cpp:643 (rng-helper): Verdict: CONFIRMED (0x611931 calls the original int helper). Class: keep-original.

## 0x6122C0 CPopulation::RemoveAllRandomPeds  (gr_deviations = 1)
Original: ped pool descending; `if (CPed::CanBeDeleted(ped))` (0x5DF100) delete it.
Row Population.cpp:761 (pool-iteration):
- Original bytes: descending 0x6122E0-0x612317. Deleting frees slots; CPool's first-free hint ends at the lowest freed index either way,
  and nothing random is consumed.
- Verdict: REFUTED (order not observable). Class: harmless.

## 0x612710 CPopulation::AddPed  (gr_deviations = 1)
Row Population.cpp:872 (NOTSA_UNREACHABLE for an unhandled ped type):
- Original bytes: type - 4 > 0x1B (`ja 0x6129D1`) or a table entry pointing there: `esi = [esp+0x58]` = the 4th argument (makeWander, a
  bool) used as the CPed* for the following `SetPosn` writes (0x6129D1-0x6129FF): an access violation at address 0x14 / 0x15.
- Verdict: REFUTED (both fail; no defined behaviour to preserve). Class: harmless.

## 0x613180 CPopulation::ChooseCivilianCoupleOccupations  (gr_deviations = 2)
Original (`__cdecl void (int* husband, int* wife)`): `a = 1, b = 0`; if `CWeather::WeatherRegion == 2` and `rand() & 0x10`: `(a, b) =
(rand() & 0x20) ? (1, 1) : (0, 0)`. `*husband = ChooseCivilianOccupation(a, b, -1, -1, -1, 0, 1, 0, 0)` (0x612F90). If `*husband >= 0`:
`*wife = ChooseCivilianOccupation(b, a, -1, *husband, ms_modelInfoPtrs[*husband]->+0x2C (stat type), 0, 1, 0, 0)`. Then if `*wife < 0`
both = -1; else if exactly one of the two models has stat type 40 (SKATER, `+0x2C`) both = -1 (0x613214-0x61324B).
Rows Population.cpp:1056 and :1062 (`#ifdef FIX_BUGS` / `#endif`, commented out):
- Verdict: REFUTED (the block is commented out; gr compiles the `#else` call order (a, b) / (b, a) as the original). Class: keep-original.
Found in passing (CONFIRMED, keep-original): (1) the original passes the husband's model and stat type as arguments 4 and 5 of the second
ChooseCivilianOccupation call; gr passes the defaults. (2) The original never initialises `*wife` before the checks: when the husband
choice fails, `*wife` keeps the caller's value and, if that is >= 0, the code reads `ms_modelInfoPtrs[-1]` (0x61321D); gr sets both to -1
first.

## 0x6133F0 CPopulation::CreateWaitingCoppers  (gr_deviations = 2)
Row Population.cpp:1124 (assert): Verdict: REFUTED (debug only). Class: harmless.
Rows Population.cpp:1182 / :1183 (rng-helper, cop ped heading and distance):
- Original bytes: per try i = 0..19 (counters `F18 = i`, `F1C = -i`, loop end 0x6139B6-0x6139C3): `dist = (float)rand() * 3.0518509e-05
  [0x858C7C]`, `dist = dist + dist + 8.0f [0x859000]` (0x6136F3 `fadd st0,st0`); `lo = (float)(-i) * 0.2f [0x858CC4]` (f32);
  `h = ((float)rand() * 3.0518509e-05) * ((float)i * 0.2f - lo) + lo + heading` (0x613716-0x61376D, the inline `(max-min)*t + min` form);
  `x = sin(h) * dist + createAt.x`, `y = cos(h) * dist + createAt.y`.
- Verdict: CONFIRMED (gr's helper uses `max*t + min*(1-t)`, giving different rounding; the distance formula is `2t + 8` here).
  Class: keep-original.

## 0x613CD0 CPopulation::PlaceMallPedsAsStationaryGroup  (gr_deviations = 1)
Original (`__cdecl void (const CVector* pos)`): only if `CGame::currArea == 4`: `m = ChooseCivilianOccupation(0, 0, -1, -1, -1, 0, 1, 0, 0)`;
`type = ms_modelInfoPtrs[m]->+0x28`; `n = ms_minGangSize [0x86C6C4] + ftol((float)(rand() & 0xFFFF) * 3.0517578e-05 [0x858B14] *
(float)(ms_maxGangSize [0x86C6C8] - ms_minGangSize))`; `CPedGroupPlacer{}.PlaceGroup(type, n, pos, 2)` (0x5FD810).
Row Population.cpp:1333 (rng-helper `GetRandomNumberInRange(1, 5)`):
- Original bytes: the inline int helper formula with the globals (initially 1 and 5).
- Verdict: CONFIRMED (helper formula; gr also hard-codes 1 and 5 instead of reading CPedGroupPlacer::ms_minGangSize / ms_maxGangSize).
  Class: keep-original.

## 0x6143E0 CPopulation::FindDistanceToNearestPedOfType  (gr_deviations = 2)
Original (`__cdecl float (int type, CVector pos)` -> ST0): best = 10000000.0f [0x86D270]; ped pool descending; for peds with
`+0x598 == type`: `d = sqrt((dz*dz + dy*dy) + dx*dx)` (d = pos - pedPos), stored f32; if `d < best` (strict) best = d (the f32 value).
Return best.
Row Population.cpp:1446 (NOTSA PED_TYPE_NONE wildcard): Verdict: REFUTED for normal types (extra case only). Class: harmless.
Row Population.cpp:1449 (pool-iteration):
- Original bytes: descending 0x614410-0x61447D; a minimum is order independent.
- Verdict: REFUTED for the order. Class: harmless. Note: gr takes the minimum of squared distances and one sqrt at the end; the original
  takes the minimum of f32-rounded square roots (x87 order above) - equal except for rounding edge cases.

## 0x6144B0 CPopulation::PickRiotRoadBlockCar  (gr_deviations = 1)
Original (`__cdecl int ()`): `base = ftol((float)(rand() & 0xFFFF) * 3.0517578e-05 [0x858B14] * 10.0f [0x85862C])` (inline int helper,
range 0..9); for i = 0..9: `m = m_LoadedGangCars[(i + base) % 10].PickRandomCar(0, 0)` (0x611C50, groups 0xC0EA28 stride 0x2E); return m if
>= 0. Then scan m_AppropriateLoadedCars (0xC0E9F8) and then m_InAppropriateLoadedCars (0xC0E9C8), 23 i16 entries each, FORWARD: return the
first entry whose `ms_modelInfoPtrs[m]->+0x4D` is not 5, 7 or 8. The `jl` after `test edx, edx` tests the byte offset, never the model, so
negative (empty, -1) entries are NOT skipped and read `ms_modelInfoPtrs[-1]` (0x614512-0x61451B). Fallback `GetDefaultCopCarModel(1)`.
Row Population.cpp:1490 (rng-helper):
- Verdict: CONFIRMED (inline original formula vs gr helper). Class: keep-original.
Found in passing (CONFIRMED, keep-original): gr scans the groups backwards and skips MODEL_INVALID entries; the original scans forwards
and does not skip them.

## 0x6160A0 CPopulation::ManageAllPopulation  (gr_deviations = 2)
Original: `c = FindPlayerCentreOfWorld(PlayerInFocus)` (copy); `t = FindPlayerPed(0)->intel->GetTaskHold(0)` (0x600FF0); held = (t &&
t->GetTaskType() (vtable +0x10) == 0x133 && (t+8)->type (`+0x36 & 7`) == 4) ? t+8 : NULL. Object pool descending: `ManageObject(obj, &c)`
(0x615DC0) unless obj == held; dummy pool descending: `ManageDummy(d, &c)` (0x616000).
Row Population.cpp:1731 (notsa dyn_cast helper): Verdict: REFUTED (same entity excluded; only objects are compared). Class: harmless.
Rows Population.cpp:1739 / :1745 (pool-iteration):
- Original bytes: both loops descending (0x616118-0x616144, 0x616158-0x61617D). ManageObject / ManageDummy convert objects to dummies and
  back, freeing and allocating pool slots inside the loop, so the visiting order changes which slots the converted entities get.
- Verdict: CONFIRMED (order differs and is observable through pool slot reuse). Class: keep-original (descending).

## 0x616190 CPopulation::ManagePopulation  (gr_deviations = 1)
Original: `b = m_FrameCounter & 0x1F` (low byte); objects: slots `[b*size/32, (b+1)*size/32)` ASCENDING (signed `/32`); dummies the same with
the dummy pool size; then the ped pool DESCENDING with `ManagePed(ped, &centre)` (0x611FC0).
Row Population.cpp:1786 (pool-iteration, peds):
- Original bytes: descending 0x6162C0-0x6162E8. ManagePed can remove peds and consumes state per ped; the visiting order is observable.
- Verdict: CONFIRMED. Class: keep-original (descending; the object/dummy batches stay ascending as in gr).

## 0x616300 CPopulation::RemovePedsIfThePoolGetsFull  (gr_deviations = 1)
Original: return unless `(m_FrameCounter & 7) == 5`; `free = size - used` (used = slots without bit 7); return if `free >= 8`. Ped pool
descending: only peds with `CanBeDeleted()` (0x5DF100); camera position = TheCamera matrix (0xB6F03C) ? matrix pos : placement 0xB6F02C;
`d = sqrt((dz*dz + dx*dx) + dy*dy)` (d = cam - pedPos; z, x, y order at 0x6163C4-0x6163D4); if `d < best` (strict, initial 10000000.0f
imm 0x4B189680): best = d (f32), cand = ped. If cand: delete it.
Row Population.cpp:1803 (pool-iteration):
- Original bytes: as above.
- Verdict: CONFIRMED, beyond the order: gr removes the nearest of ALL peds (no CanBeDeleted filter, so it can delete mission peds or the
  player) and keeps the lowest index on ties; the original only considers deletable peds and keeps the highest index on ties. Class:
  keep-original.

## 0x616420 CPopulation::ConvertAllObjectsToDummyObjects  (gr_deviations = 1)
Original: object pool descending; if `byte +0x13C == 1` (m_nObjectType): `ConvertToDummyObject(obj)` (0x614670;
the pool pointer is re-read after each call).
Row Population.cpp:1812 (pool-iteration):
- Original bytes: descending 0x616437-0x616469; ConvertToDummyObject frees object slots inside the loop.
- Verdict: CONFIRMED (order observable through pool slot reuse). Class: keep-original. Note: gr tests `m_nObjectType != OBJECT_GAME`;
  the original tests the byte value 1; OBJECT_GAME = 1 (Object.h:14), so the test is the same.

## 0x616650 CPopulation::Update  (gr_deviations = 1)
Original (`__cdecl void (bool generatePeds)`): `switch ((int16)WeatherRegion)` unsigned `ja` > 4 skips; table 0x616844: 0, 1, 4 ->
CurrentWorldZone = 0; 2 -> 1; 3 -> 2. Then the rest (replay check, ManagePopulation, RemovePedsIfThePoolGetsFull, countdown, totals,
AddToPopulation / GeneratePedsAtAttractors) as gr.
Row Population.cpp:1878 (NOTSA_UNREACHABLE in the region switch):
- Original bytes: any other region value leaves CurrentWorldZone unchanged (0x61665B).
- Verdict: CONFIRMED (gr aborts). Class: keep-original.
