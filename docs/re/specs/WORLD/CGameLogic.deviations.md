# CGameLogic: gta-reversed deviation notes (WORLD)

Scope: functions that gta-reversed provides and that have `gr_deviations > 0` in `catalog/state.csv`. Each `##` checks the flagged rows of `catalog/gr-deviations.csv` against the original 1.0 US bytes (bundle disassembly is the authority). CONFIRMED = gr really differs; REFUTED = gr matches the original or the marker is not a behavioural difference; UNCLEAR = open question. Class keep-original = the port must do what the original does; harmless = no observable difference. "Found in passing" items are differences between gr and the bytes that the CAT-14 rows did not flag.

Common notation: `rand` = CRT rand 0x821b1e, `_ftol` = 0x821b40 (truncation), 0x407180 / 0x41bd90 = the original CGeneral::GetRandomNumberInRange int/float helpers (gr's helpers differ, see docs/re/cat14-gr-deviations.md finding 1). Pool loops: "descending" means slot size-1 down to 0 (flag byte bit 7 = free slot), which is the opposite of gr's `GetAllValid()`.

## 0x4418e0 CGameLogic::CalcDistanceToForbiddenTrainCrossing  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
Args (cdecl, bytes): point CVector +4..+0xc (`[esp+0x24..0x2c]` in the body), moveSpeed CVector +0x10..+0x18 (`[esp+0x30..0x38]`), bool ignoreMoveSpeed +0x1c (BL), CVector* out +0x20 (ESI). Returns ST0.
- gr `source/game_sa/GameLogic.cpp:49` (original-comment): vanilla used a 3D dot product with crossing z = 0; gr uses a 2D dot product instead.
- gr `GameLogic.cpp:53` (original-comment): `outDistance = CVector{dir2D, 0.f}; // Original code here used z = -point.z`.
  - Original bytes, per crossing C (closest starts at 100000.0f, stored at [esp+8], 0x4418ea):
    - `dir = (C.x - p.x, C.y - p.y, -p.z)` (`fld C.x; fsub p.x`, `fld C.y; fsub p.y`, `fld p.z; fchs`), dy and -pz saved as f32.
    - `dot = (m.z * (-p.z) + m.y * dir.y) + m.x * dir.x` (x87 order at 0x441937-0x44194b), `fcomp 0.0 [0x858b50]`: if `dot > 0` (ordered) go on; else go on only if `ignoreMoveSpeed` (0x44195e-0x441962).
    - `dist = sqrt((p.y - C.y)^2 + (p.x - C.x)^2)` (note: computed from point minus crossing, `dy*dy` first then `+ dx*dx`, 0x441964-0x441982); if `dist < closest` (ordered, `test ah,5; jp` skip): `closest = dist` (f32), `*out = (dir.x, dir.y, -p.z)`.
  - Verdict (both rows): CONFIRMED. The original uses the 3D dot product including `-p.z * moveSpeed.z`, and writes `out.z = -p.z`. gr uses 2D and `out.z = 0`.
  - Class: keep-original.
- Found in passing (keep-original, CONFIRMED):
  1. Branch structure. gr uses `if (stat == 0) .. else if (stat == 1) .. else if (stat < 1)`. The original reads `CStats::GetStatValue(0xb5)` (0x558e40) three times and runs three independent tests: crossing 1 if `stat == 0.0` (`test ah,0x44; jp` skip, 0x44190a), crossing 2 if `stat == 1.0` [0x858624] (0x4419de), crossing 3 if `stat <= 1.0` (`test ah,0x41; jp` skip, 0x441aac: taken only for `>` or unordered). So with stat 0 crossings 1 and 3 are both tested, with stat 1 crossings 2 and 3.
  2. Crossing 3 is at (2770.0 [0x859824] = 0x452d2000, 277.0 [0x859820]); gr has 2270.0. Crossings 1 and 2: (82.0 [0x859834], -1021.0 [0x859830]) and (-1568.0 [0x85982c], 537.0 [0x859828]) match gr.
  3. Crossing 1 compares dist against the constant 100000.0f [0x858ef8] (`fcom`, 0x441988); crossings 2 and 3 against the stored closest [esp+8]. Return: crossing-3 accept path returns the new dist in ST0 (0x441b35); otherwise `fld [esp+8]` (0x441b5d).

## 0x441b70 CGameLogic::FindCityClosestToPoint  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/GameLogic.cpp:113` (notsa): `std::pair<float, eLevelName> closest{FLT_MAX, NUM_LEVELS}; // NOTSA`, then a loop over LS, SF, LV in that order with `d < closest.first`.
- gr `GameLogic.cpp:121` (notsa-unreachable): `if (closest.second == NUM_LEVELS) NOTSA_UNREACHABLE();`
  - Original bytes (fully unrolled, no sentinel): `ecx = 1` (0x441b75). dLS = sqrt((px-1670.0[0x85984c])^2 + (py-(-1137.0)[0x859848])^2) kept in ST (0x441b71-0x441b98; x87 computes `dy*dy + dx*dx`: `fld st0; fmul st1` on dy, then dx*dx, `faddp`). dLV = same with 2161.0 [0x859844], 2140.0 [0x859840] (0x441b9a-0x441bbc), stored with `fst [esp]` (f32). `fcomp st1` compares dLV with dLS; `test ah,5; jp` (0x441bc6): if dLV < dLS (ordered) then `ecx = 3`, best = reload dLV from [esp] (0x441bcb-0x441bd2); otherwise keep dLS. dSF with -1810.0 [0x85983c], 884.0 [0x859838] (0x441bd6-0x441bf8); `fcomp st1` vs best; if dSF < best (ordered) return 2 (0x441c03), else return ECX (0x441c0a).
  - Verdict (both rows): CONFIRMED. (a) Compare order is LS, then LV, then SF (gr: LS, SF, LV), which matters on exact ties: dSF == dLV < dLS gives LV (3) in the original, SF (2) in gr. (b) With NaN coordinates every compare is unordered and the original returns 1 (LS); gr hits NOTSA_UNREACHABLE.
  - Class: keep-original.

## 0x441d30 CGameLogic::RestorePedsWeapons  (gr_deviations = 1)
Rows checked: 1 (CONFIRMED 1, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/GameLogic.cpp:350` (fix-bugs-comment): `if (rng::all_of(weaponInfo.GetModels(), IsModelLoaded)) // FIX_BUGS: They checked modelId1 twice`.
  - Original bytes: body 0x1560f90. `ClearWeapons()` (0x5e6320); for each of the 13 SavedWeapons (0x96a9b8, stride 0x1c, until 0x96ab24): `m1 = GetWeaponInfo(type, 1)->m_nModelId1 (+0xc)`, `m2 = GetWeaponInfo(type, 1)->m_nModelId2 (+0x10)` (0x743c60 called twice); `if (m1 != -1 && ms_aInfoForModel[m1].m_nLoadState (0x8e4cd0 + m1*0x14) != 1) skip`; `if (m2 != -1 && ms_aInfoForModel[m1].m_nLoadState != 1) skip` (the second test indexes with m1 again, 0x1560fdd); else `GiveWeapon(type, totalAmmo (+0xc), 1)` (0x5e6080).
  - Verdict: CONFIRMED. The original never checks model 2's load state, so a weapon whose second model is not loaded is still given; gr skips it.
  - Class: keep-original.

## 0x442ad0 CGameLogic::Update  (gr_deviations = 2)
Rows checked: 2 (CONFIRMED 2, REFUTED 0, UNCLEAR 0).
- gr `source/game_sa/GameLogic.cpp:705` (rng-helper): `if (CGeneral::GetRandomNumberInRange(0, 4))` (busted audio).
  - Original bytes: `CGeneral::GetRandomNumberInRange_int32(0 /*ebp*/, 4)` (0x407180, 0x44311f); non-zero -> `Players[0].m_nBustedAudioStatus (0xb7ceec) = 2`; zero -> status 1 and `m_nLastBustMessageNumber (u16 0xb7ceee) = (u16 % 28) + 1` (0x44312b-0x443142).
  - Verdict: CONFIRMED (gr's integer helper vs 0x407180). Class: keep-original (call the ported 0x407180).
- gr `GameLogic.cpp:726` (notsa-unreachable): `default: NOTSA_UNREACHABLE(); // NOTSA, SA returns 100.` in the arrest-fee switch.
  - Original bytes: `switch (wanted level ([ped wanted data]+0x2c) - 2)` through the table at 0x4438d4 for levels 2..6 (200, 400, 600, 900, 1500; 0x4431a7-0x4431c8); every other value, including level 1 and 0, gives 100 (`ja 0x4431ca`, `mov ecx,0x64`). The fee is computed before the penalty/jail-card tests. Penalty: `if (money (0xb7ce50) > 0) money = max(money - fee, 0)` (`sets/dec/and` idiom, 0x4431ea-0x443200), then `ClearWeapons`.
  - Verdict: CONFIRMED (the original returns 100 for out-of-table levels; gr's comment is right, its code is not).
  - Class: keep-original.
